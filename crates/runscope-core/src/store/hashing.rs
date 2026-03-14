use crate::error::RunScopeError;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn sha256_hex_str(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn sha256_hex_file(path: &Path) -> Result<String, RunScopeError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn sha256_hex_dir(path: &Path) -> Result<String, RunScopeError> {
    let mut files = Vec::new();
    collect_files(path, path, &mut files)?;
    files.sort();

    let mut hasher = Sha256::new();
    for relative_path in files {
        hasher.update(relative_path.to_string_lossy().as_bytes());
        let mut file = fs::File::open(path.join(&relative_path))?;
        let mut buffer = [0_u8; 8192];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn sha256_hex_path(path: &Path) -> Result<String, RunScopeError> {
    if path.is_file() {
        return sha256_hex_file(path);
    }
    sha256_hex_dir(path)
}

pub fn canonical_json_sha256<T: Serialize>(value: &T) -> Result<String, RunScopeError> {
    Ok(sha256_hex_str(&serde_json::to_string(value)?))
}

fn collect_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), RunScopeError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else {
            files.push(
                path.strip_prefix(root)
                    .map_err(|_| {
                        RunScopeError::ManifestValidation("invalid source path".to_string())
                    })?
                    .to_path_buf(),
            );
        }
    }
    Ok(())
}
