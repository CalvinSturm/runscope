use crate::adapters::faceapp::FaceappAdapter;
use crate::adapters::localagent::LocalAgentAdapter;
use crate::adapters::traits::RunAdapter;
use crate::adapters::videoforge::VideoforgeAdapter;
use crate::error::RunScopeError;
use std::path::Path;

pub fn select_adapter(
    explicit_adapter: Option<&str>,
    artifact_dir: &Path,
) -> Result<Box<dyn RunAdapter>, RunScopeError> {
    match explicit_adapter {
        Some("auto") | None => auto_detect_adapter(artifact_dir),
        Some("localagent") => Ok(Box::new(LocalAgentAdapter)),
        Some("videoforge") => Ok(Box::new(VideoforgeAdapter)),
        Some("faceapp") => Ok(Box::new(FaceappAdapter)),
        Some(_) => Err(RunScopeError::AdapterNotDetected),
    }
}

fn auto_detect_adapter(artifact_dir: &Path) -> Result<Box<dyn RunAdapter>, RunScopeError> {
    let localagent = LocalAgentAdapter;
    let videoforge = VideoforgeAdapter;
    let faceapp = FaceappAdapter;

    let localagent_detected = localagent.detect(artifact_dir)?;
    let videoforge_detected = videoforge.detect(artifact_dir)?;
    let faceapp_detected = faceapp.detect(artifact_dir)?;
    let detected_count = [localagent_detected, videoforge_detected, faceapp_detected]
        .into_iter()
        .filter(|detected| *detected)
        .count();

    if detected_count > 1 {
        return Err(RunScopeError::AdapterAmbiguous);
    }
    if localagent_detected {
        return Ok(Box::new(localagent));
    }
    if videoforge_detected {
        return Ok(Box::new(videoforge));
    }
    if faceapp_detected {
        return Ok(Box::new(faceapp));
    }
    Err(RunScopeError::AdapterNotDetected)
}
