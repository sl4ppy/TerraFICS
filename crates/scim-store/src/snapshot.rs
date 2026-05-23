//! Snapshot DAG.
//!
//! One node per saved state. P1.4 creates exactly one snapshot per
//! `import_save` call. P2 will add child snapshots per edit batch.

use rusqlite::{params, Connection};

use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRow {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub created_at: i64,
    pub label: String,
    pub note: Option<String>,
    pub source_sav_path: Option<String>,
}

/// Add the given `actor_id` to `snapshot_id`'s membership list.
pub fn add_actor_to_snapshot(conn: &Connection, snapshot_id: i64, actor_id: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO snapshot_actor(snapshot_id, actor_id) VALUES (?, ?)",
        params![snapshot_id, actor_id],
    )?;
    Ok(())
}

/// Create a new snapshot row. Returns the auto-assigned id.
pub fn create_snapshot(
    conn: &Connection,
    parent_id: Option<i64>,
    created_at: i64,
    label: &str,
    note: Option<&str>,
    source_sav_path: Option<&str>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO snapshot(parent_id, created_at, label, note, source_sav_path)
         VALUES (?, ?, ?, ?, ?)",
        params![parent_id, created_at, label, note, source_sav_path],
    )?;
    Ok(conn.last_insert_rowid())
}

/// List all snapshots ordered by `created_at` ascending.
pub fn list_snapshots(conn: &Connection) -> Result<Vec<SnapshotRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, created_at, label, note, source_sav_path
           FROM snapshot ORDER BY created_at",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(SnapshotRow {
            id: r.get(0)?,
            parent_id: r.get(1)?,
            created_at: r.get(2)?,
            label: r.get(3)?,
            note: r.get(4)?,
            source_sav_path: r.get(5)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    #[test]
    fn create_and_list_snapshots() {
        let db = Db::open_in_memory().unwrap();
        let id1 = create_snapshot(db.conn(), None, 1_000, "first", None, None).unwrap();
        let id2 = create_snapshot(
            db.conn(),
            Some(id1),
            2_000,
            "second",
            Some("a note"),
            Some("/some/path.sav"),
        )
        .unwrap();
        let snaps = list_snapshots(db.conn()).unwrap();
        assert_eq!(snaps.len(), 2);
        assert_eq!(snaps[0].id, id1);
        assert_eq!(snaps[1].parent_id, Some(id1));
        assert_eq!(snaps[1].note.as_deref(), Some("a note"));
        let _ = id2;
    }
}
