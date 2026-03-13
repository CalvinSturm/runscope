use runscope_core::error::RunScopeError;
use runscope_core::services::AppPaths;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub struct AppState {
    pub paths: AppPaths,
}

impl AppState {
    pub fn new(app: &AppHandle) -> Result<Self, RunScopeError> {
        let data_dir = app_data_dir(app)?;
        fs::create_dir_all(&data_dir)?;
        Ok(Self {
            paths: AppPaths {
                db_path: data_dir.join("runscope.sqlite"),
                data_dir,
            },
        })
    }
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, RunScopeError> {
    if let Ok(path) = app.path().app_local_data_dir() {
        return Ok(path);
    }
    if let Ok(path) = app.path().app_data_dir() {
        return Ok(path);
    }
    Err(RunScopeError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "unable to resolve app data directory",
    )))
}
