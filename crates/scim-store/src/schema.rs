//! SQL schema for the project database. Applied idempotently on `Db::open`.
//!
//! Schema mirrors design spec §5.1.

use rusqlite::Connection;

use crate::error::Result;

const SCHEMA_SQL: &str = r"
CREATE TABLE IF NOT EXISTS blob (
    hash BLOB PRIMARY KEY NOT NULL,
    zstd_data BLOB NOT NULL
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS actor (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path_name TEXT NOT NULL,
    class_name TEXT NOT NULL,
    level TEXT NOT NULL,
    transform BLOB,
    blob_hash BLOB NOT NULL,
    FOREIGN KEY (blob_hash) REFERENCES blob(hash)
);

CREATE INDEX IF NOT EXISTS actor_path_name_idx ON actor(path_name);
CREATE INDEX IF NOT EXISTS actor_class_name_idx ON actor(class_name);

CREATE TABLE IF NOT EXISTS snapshot (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id INTEGER,
    created_at INTEGER NOT NULL,
    label TEXT NOT NULL,
    note TEXT,
    source_sav_path TEXT,
    FOREIGN KEY (parent_id) REFERENCES snapshot(id)
);

CREATE TABLE IF NOT EXISTS snapshot_actor (
    snapshot_id INTEGER NOT NULL,
    actor_id INTEGER NOT NULL,
    PRIMARY KEY (snapshot_id, actor_id),
    FOREIGN KEY (snapshot_id) REFERENCES snapshot(id),
    FOREIGN KEY (actor_id) REFERENCES actor(id)
);

CREATE TABLE IF NOT EXISTS header (
    snapshot_id INTEGER PRIMARY KEY NOT NULL,
    json TEXT NOT NULL,
    FOREIGN KEY (snapshot_id) REFERENCES snapshot(id)
);
";

pub fn apply_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_schema_to_in_memory_db() {
        let conn = Connection::open_in_memory().unwrap();
        apply_schema(&conn).unwrap();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let names: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .filter_map(std::result::Result::ok)
            .collect();
        assert!(names.iter().any(|n| n == "blob"));
        assert!(names.iter().any(|n| n == "actor"));
        assert!(names.iter().any(|n| n == "snapshot"));
        assert!(names.iter().any(|n| n == "snapshot_actor"));
        assert!(names.iter().any(|n| n == "header"));
    }

    #[test]
    fn apply_schema_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        apply_schema(&conn).unwrap();
        apply_schema(&conn).unwrap();
        apply_schema(&conn).unwrap();
    }
}
