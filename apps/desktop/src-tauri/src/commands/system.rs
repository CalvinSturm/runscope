use std::path::Path;
use std::process::Command;

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let path_ref = Path::new(&path);
    if !path_ref.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    open_in_file_manager(path_ref, false)
}

#[tauri::command]
pub fn reveal_path(path: String) -> Result<(), String> {
    let path_ref = Path::new(&path);
    if !path_ref.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    open_in_file_manager(path_ref, true)
}

fn open_in_file_manager(path: &Path, reveal: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("explorer");
        if reveal && path.is_file() {
            command.arg("/select,").arg(path);
        } else {
            command.arg(path);
        }

        command
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        if reveal {
            command.arg("-R");
        }
        command.arg(path);
        command
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let target = if reveal && path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path
        };

        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}
