use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ResolvedAppPaths {
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
}

pub fn resolve_app_paths(
    data_dir_override: Option<PathBuf>,
    db_path_override: Option<PathBuf>,
) -> ResolvedAppPaths {
    let data_dir = data_dir_override
        .or_else(data_dir_from_env)
        .unwrap_or_else(default_data_dir);
    let db_path = db_path_override.unwrap_or_else(|| data_dir.join("runscope.sqlite"));
    ResolvedAppPaths { data_dir, db_path }
}

fn data_dir_from_env() -> Option<PathBuf> {
    env::var("RUNSCOPE_DATA_DIR").ok().map(PathBuf::from)
}

pub fn default_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(value) = env::var("LOCALAPPDATA") {
            return PathBuf::from(value).join("RunScope");
        }
        if let Ok(value) = env::var("APPDATA") {
            return PathBuf::from(value).join("RunScope");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("RunScope");
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Ok(xdg_data_home) = env::var("XDG_DATA_HOME") {
            return PathBuf::from(xdg_data_home).join("runscope");
        }
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("runscope");
        }
    }

    PathBuf::from(".runscope")
}
