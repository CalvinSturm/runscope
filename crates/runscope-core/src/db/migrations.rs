use crate::error::RunScopeError;
use rusqlite::Connection;

const INIT_SQL: &str = include_str!("../../migrations/0001_init.sql");

pub fn apply_migrations(conn: &Connection) -> Result<(), RunScopeError> {
    conn.execute_batch(INIT_SQL)?;
    Ok(())
}
