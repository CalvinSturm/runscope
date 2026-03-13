use crate::adapters::SourceFile;
use crate::domain::{ArtifactRecord, RunManifestV1};
use crate::error::RunScopeError;
use crate::store::hashing::sha256_hex_file;
use crate::store::layout::{ensure_run_layout, RunLayoutPaths};
use std::fs;
use std::path::{Path, PathBuf};

pub struct ArtifactStore {
    data_dir: PathBuf,
}

pub struct CopiedArtifact {
    pub role: String,
    pub rel_path: String,
    pub media_type: String,
    pub sha256: String,
    pub size_bytes: u64,
}

impl ArtifactStore {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    pub fn prepare_run_layout(
        &self,
        project_slug: &str,
        ingested_at: &str,
        run_id: &str,
    ) -> Result<RunLayoutPaths, RunScopeError> {
        ensure_run_layout(&self.data_dir, project_slug, ingested_at, run_id)
    }

    pub fn copy_source_files(
        &self,
        layout: &RunLayoutPaths,
        files_to_copy: &[SourceFile],
    ) -> Result<Vec<CopiedArtifact>, RunScopeError> {
        let mut copied = Vec::with_capacity(files_to_copy.len());
        for file in files_to_copy {
            validate_target_rel_path(&file.target_rel_path)?;
            let destination = layout.run_root.join(&file.target_rel_path);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&file.source_path, &destination)?;
            let metadata = fs::metadata(&destination)?;
            copied.push(CopiedArtifact {
                role: file.role.clone(),
                rel_path: file.target_rel_path.clone(),
                media_type: file.media_type.clone(),
                sha256: sha256_hex_file(&destination)?,
                size_bytes: metadata.len(),
            });
        }
        Ok(copied)
    }

    pub fn write_run_manifest(
        &self,
        layout: &RunLayoutPaths,
        manifest: &RunManifestV1,
    ) -> Result<(), RunScopeError> {
        fs::write(
            &layout.run_json_path,
            serde_json::to_string_pretty(manifest)?,
        )?;
        Ok(())
    }
}

pub fn copied_artifacts_to_records(copied: Vec<CopiedArtifact>) -> Vec<ArtifactRecord> {
    copied
        .into_iter()
        .map(|artifact| ArtifactRecord {
            role: artifact.role,
            rel_path: artifact.rel_path,
            media_type: artifact.media_type,
            sha256: Some(artifact.sha256),
            size_bytes: Some(artifact.size_bytes),
        })
        .collect()
}

pub fn infer_media_type_from_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "json" => "application/json",
        "html" | "htm" => "text/html",
        "txt" | "log" | "md" => "text/plain",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "mp4" => "video/mp4",
        "csv" => "text/csv",
        _ => "application/octet-stream",
    }
}

fn validate_target_rel_path(target_rel_path: &str) -> Result<(), RunScopeError> {
    let path = Path::new(target_rel_path);
    if path.is_absolute() {
        return Err(RunScopeError::ManifestValidation(format!(
            "artifact target path must be relative: {target_rel_path}"
        )));
    }
    for component in path.components() {
        if matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        ) {
            return Err(RunScopeError::ManifestValidation(format!(
                "artifact target path must stay under the run root: {target_rel_path}"
            )));
        }
    }
    Ok(())
}
