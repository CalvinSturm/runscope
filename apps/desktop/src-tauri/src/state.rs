use runscope_core::error::RunScopeError;
use runscope_core::services::{resolve_app_paths, AppPaths};
use std::fs;

pub struct AppState {
    pub paths: AppPaths,
}

impl AppState {
    pub fn new() -> Result<Self, RunScopeError> {
        let resolved = resolve_app_paths(None, None);
        fs::create_dir_all(&resolved.data_dir)?;
        Ok(Self {
            paths: AppPaths {
                db_path: resolved.db_path,
                data_dir: resolved.data_dir,
            },
        })
    }
}
