#[derive(thiserror::Error, Debug)]
pub enum RunScopeError {
    #[error("adapter not detected")]
    AdapterNotDetected,
    #[error("adapter ambiguous")]
    AdapterAmbiguous,
    #[error("manifest validation failed: {0}")]
    ManifestValidation(String),
    #[error("duplicate ingest: existing run {0}")]
    DuplicateIngest(String),
    #[error("run not found: {0}")]
    RunNotFound(String),
    #[error("baseline scope invalid: {0}")]
    BaselineScopeInvalid(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}
