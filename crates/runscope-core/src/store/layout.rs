use crate::error::RunScopeError;
use std::fs;
use std::path::{Path, PathBuf};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct RunLayoutPaths {
    pub run_root: PathBuf,
    pub raw_dir: PathBuf,
    pub derived_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub attachments_dir: PathBuf,
    pub run_json_path: PathBuf,
}

pub fn managed_run_root(
    data_dir: &Path,
    project_slug: &str,
    ingested_at: &str,
    run_id: &str,
) -> Result<PathBuf, RunScopeError> {
    let timestamp = OffsetDateTime::parse(ingested_at, &Rfc3339).map_err(|_| {
        RunScopeError::ManifestValidation("source.ingested_at must be RFC3339 UTC".to_string())
    })?;

    Ok(data_dir
        .join("artifacts")
        .join(project_slug)
        .join(format!("{:04}", timestamp.year()))
        .join(format!("{:02}", timestamp.month() as u8))
        .join(run_id))
}

pub fn ensure_run_layout(
    data_dir: &Path,
    project_slug: &str,
    ingested_at: &str,
    run_id: &str,
) -> Result<RunLayoutPaths, RunScopeError> {
    let run_root = managed_run_root(data_dir, project_slug, ingested_at, run_id)?;
    let paths = RunLayoutPaths {
        raw_dir: run_root.join("raw"),
        derived_dir: run_root.join("derived"),
        logs_dir: run_root.join("logs"),
        attachments_dir: run_root.join("attachments"),
        run_json_path: run_root.join("run.json"),
        run_root,
    };
    fs::create_dir_all(&paths.raw_dir)?;
    fs::create_dir_all(&paths.derived_dir)?;
    fs::create_dir_all(&paths.logs_dir)?;
    fs::create_dir_all(&paths.attachments_dir)?;
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn artifact_store_layout_is_deterministic() {
        let data_dir = PathBuf::from("C:/runscope-data");
        let path = managed_run_root(
            &data_dir,
            "localagent",
            "2026-03-05T17:20:31Z",
            "01JNP8M2A4HD7Q7RAN6TKPS9YF",
        )
        .unwrap();

        assert_eq!(
            path,
            PathBuf::from(
                "C:/runscope-data/artifacts/localagent/2026/03/01JNP8M2A4HD7Q7RAN6TKPS9YF"
            )
        );
        assert_eq!(
            path,
            managed_run_root(
                &data_dir,
                "localagent",
                "2026-03-05T17:20:31Z",
                "01JNP8M2A4HD7Q7RAN6TKPS9YF",
            )
            .unwrap()
        );
    }
}
