use crate::domain::{AdapterWarning, RunManifestV1};
use crate::error::RunScopeError;
use std::path::{Path, PathBuf};

pub trait RunAdapter {
    fn name(&self) -> &'static str;
    fn detect(&self, artifact_dir: &Path) -> Result<bool, RunScopeError>;
    fn parse(&self, artifact_dir: &Path) -> Result<ParsedRun, RunScopeError>;
}

#[derive(Debug, Clone)]
pub struct ParsedRun {
    pub manifest: RunManifestV1,
    pub files_to_copy: Vec<SourceFile>,
    pub warnings: Vec<AdapterWarning>,
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub source_path: PathBuf,
    pub target_rel_path: String,
    pub role: String,
    pub media_type: String,
}
