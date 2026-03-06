use crate::adapters::localagent::LocalAgentAdapter;
use crate::adapters::traits::RunAdapter;
use crate::error::RunScopeError;
use std::path::Path;

pub fn select_adapter(
    explicit_adapter: Option<&str>,
    artifact_dir: &Path,
) -> Result<Box<dyn RunAdapter>, RunScopeError> {
    match explicit_adapter {
        Some("auto") | None => auto_detect_adapter(artifact_dir),
        Some("localagent") => Ok(Box::new(LocalAgentAdapter)),
        Some(_) => Err(RunScopeError::AdapterNotDetected),
    }
}

fn auto_detect_adapter(artifact_dir: &Path) -> Result<Box<dyn RunAdapter>, RunScopeError> {
    let adapter = LocalAgentAdapter;
    if adapter.detect(artifact_dir)? {
        return Ok(Box::new(adapter));
    }
    Err(RunScopeError::AdapterNotDetected)
}
