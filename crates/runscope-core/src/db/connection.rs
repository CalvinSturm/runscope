use crate::error::RunScopeError;
use rusqlite::Connection;
use std::fs;
use std::path::Path;

pub fn open_connection(db_path: &Path) -> Result<Connection, RunScopeError> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(conn)
}
