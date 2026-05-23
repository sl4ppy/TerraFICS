//! Per-snapshot save-header storage.
//!
//! The full `scim_savefile::Header` is serialized via `serde_json` and stored
//! in the `header` table keyed by `snapshot_id`. This preserves every header
//! field (build version, session name, mod metadata, etc.) so a snapshot can
//! be replayed exactly even if the source `.sav` is no longer available.

use rusqlite::{params, Connection, OptionalExtension};
use scim_savefile::Header;

use crate::error::Result;

pub fn insert_header(conn: &Connection, snapshot_id: i64, header: &Header) -> Result<()> {
    let json = serde_json::to_string(header)?;
    conn.execute(
        "INSERT INTO header(snapshot_id, json) VALUES (?, ?)",
        params![snapshot_id, json],
    )?;
    Ok(())
}

pub fn read_header(conn: &Connection, snapshot_id: i64) -> Result<Option<Header>> {
    let json: Option<String> = conn
        .query_row(
            "SELECT json FROM header WHERE snapshot_id = ?",
            params![snapshot_id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(json) = json else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_str(&json)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::snapshot::create_snapshot;

    fn synth_header() -> Header {
        Header {
            save_header_type: 14,
            save_version: 46,
            build_version: 367_502,
            save_name: None,
            map_name: "Persistent_Level".to_string(),
            map_options: String::new(),
            session_name: "test".to_string(),
            play_duration_seconds: 100,
            save_date_time: 638_000_000_000_000_000,
            session_visibility: 128,
            editor_object_version: Some(40),
            mod_metadata: None,
            is_modded_save: Some(0),
            save_identifier: Some("abc".to_string()),
            is_partitioned_world: Some(0),
            save_data_hash: None,
            is_creative_mode_enabled: Some(1),
        }
    }

    #[test]
    fn roundtrip_header_json() {
        let db = Db::open_in_memory().unwrap();
        let snap = create_snapshot(db.conn(), None, 0, "x", None, None).unwrap();
        let h = synth_header();
        insert_header(db.conn(), snap, &h).unwrap();
        let read = read_header(db.conn(), snap).unwrap().unwrap();
        assert_eq!(read.map_name, "Persistent_Level");
        assert_eq!(read.session_name, "test");
        assert_eq!(read.editor_object_version, Some(40));
    }

    #[test]
    fn read_missing_snapshot_returns_none() {
        let db = Db::open_in_memory().unwrap();
        assert!(read_header(db.conn(), 999).unwrap().is_none());
    }
}
