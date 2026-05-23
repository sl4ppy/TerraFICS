//! `Db` — owned wrapper around a `rusqlite::Connection`.
//!
//! Tuning + schema are applied at open time. The public API of `scim-store`
//! takes `&Db` (or `&mut Db` for write paths) rather than `Connection`
//! directly, so callers can't bypass the invariants.

use std::path::Path;

use rusqlite::Connection;

use crate::error::Result;
use crate::{apply_schema, apply_tuning};

#[derive(Debug)]
pub struct Db {
    conn: Connection,
}

impl Db {
    /// Open (or create) a project database at `path`. Applies tuning and schema.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        apply_tuning(&conn)?;
        apply_schema(&conn)?;
        Ok(Self { conn })
    }

    /// In-memory database for tests.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        apply_tuning(&conn)?;
        apply_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Borrow the underlying connection.
    #[must_use]
    pub const fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Mutable connection access — needed for `transaction()`.
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_applies_schema() {
        let db = Db::open_in_memory().unwrap();
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='blob'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn open_creates_file_db() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.scimdb");
        {
            let _db = Db::open(&path).unwrap();
        }
        assert!(path.exists());
        let _db = Db::open(&path).unwrap();
    }
}
