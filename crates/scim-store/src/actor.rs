//! Actor row storage.
//!
//! One row per actor-version; immutable. The same `path_name` can have
//! multiple rows when P2 adds the edit path.
//!
//! `transform` is a 40-byte little-endian f32 blob: 4 rotation + 3 translation +
//! 3 scale = 10 × f32. `None` for `ObjectKind::Object` actors which have no
//! transform.

use rusqlite::{params, Connection};

use crate::blob::BlobHash;
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorRow {
    pub id: i64,
    pub path_name: String,
    pub class_name: String,
    pub level: String,
    /// 40-byte transform blob, or None for non-Actor records.
    pub transform: Option<[u8; 40]>,
    pub blob_hash: BlobHash,
}

/// Encode the 10 floats of a transform (rotation 4, translation 3, scale 3)
/// as a 40-byte little-endian blob.
#[must_use]
pub fn encode_transform(rotation: [f32; 4], translation: [f32; 3], scale: [f32; 3]) -> [u8; 40] {
    let mut buf = [0_u8; 40];
    let mut off = 0;
    for f in rotation {
        buf[off..off + 4].copy_from_slice(&f.to_le_bytes());
        off += 4;
    }
    for f in translation {
        buf[off..off + 4].copy_from_slice(&f.to_le_bytes());
        off += 4;
    }
    for f in scale {
        buf[off..off + 4].copy_from_slice(&f.to_le_bytes());
        off += 4;
    }
    buf
}

/// Decode a 40-byte transform blob back to (rotation, translation, scale).
pub fn decode_transform(blob: &[u8]) -> Result<([f32; 4], [f32; 3], [f32; 3])> {
    if blob.len() != 40 {
        return Err(Error::TransformBlobLength { found: blob.len() });
    }
    let mut off = 0;
    let mut read_f32 = || {
        let v = f32::from_le_bytes(blob[off..off + 4].try_into().unwrap());
        off += 4;
        v
    };
    let rotation = [read_f32(), read_f32(), read_f32(), read_f32()];
    let translation = [read_f32(), read_f32(), read_f32()];
    let scale = [read_f32(), read_f32(), read_f32()];
    Ok((rotation, translation, scale))
}

/// Insert a new actor row. Returns the row's auto-assigned id.
pub fn insert_actor(
    conn: &Connection,
    path_name: &str,
    class_name: &str,
    level: &str,
    transform: Option<&[u8; 40]>,
    blob_hash: BlobHash,
) -> Result<i64> {
    let transform_slice: Option<&[u8]> = transform.map(<[u8; 40]>::as_slice);
    conn.execute(
        "INSERT INTO actor(path_name, class_name, level, transform, blob_hash)
         VALUES (?, ?, ?, ?, ?)",
        params![
            path_name,
            class_name,
            level,
            transform_slice,
            blob_hash.as_array().as_slice(),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// List all actors that belong to a given snapshot, joined via `snapshot_actor`.
pub fn list_actors_in_snapshot(conn: &Connection, snapshot_id: i64) -> Result<Vec<ActorRow>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.path_name, a.class_name, a.level, a.transform, a.blob_hash
           FROM actor a
           INNER JOIN snapshot_actor sa ON sa.actor_id = a.id
           WHERE sa.snapshot_id = ?
           ORDER BY a.id",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |r| {
        let transform_vec: Option<Vec<u8>> = r.get(4)?;
        let transform = transform_vec.map(|v| {
            let mut arr = [0_u8; 40];
            arr.copy_from_slice(&v[..40.min(v.len())]);
            arr
        });
        let hash_vec: Vec<u8> = r.get(5)?;
        let mut hash_arr = [0_u8; 32];
        hash_arr.copy_from_slice(&hash_vec[..32.min(hash_vec.len())]);
        Ok(ActorRow {
            id: r.get(0)?,
            path_name: r.get(1)?,
            class_name: r.get(2)?,
            level: r.get(3)?,
            transform,
            blob_hash: BlobHash::from_array(hash_arr),
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
    use crate::blob::insert_blob_if_absent;
    use crate::db::Db;

    #[test]
    fn encode_decode_transform_roundtrip() {
        let rot = [0.1_f32, 0.2, 0.3, 1.0];
        let trans = [10.0_f32, 20.0, 30.0];
        let scale = [1.0_f32, 1.0, 1.0];
        let encoded = encode_transform(rot, trans, scale);
        let (r2, t2, s2) = decode_transform(&encoded).unwrap();
        for i in 0..4 {
            assert!((rot[i] - r2[i]).abs() < f32::EPSILON);
        }
        for i in 0..3 {
            assert!((trans[i] - t2[i]).abs() < f32::EPSILON);
            assert!((scale[i] - s2[i]).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn decode_transform_wrong_length_errors() {
        let err = decode_transform(&[0_u8; 30]).unwrap_err();
        assert!(matches!(err, Error::TransformBlobLength { found: 30 }));
    }

    #[test]
    fn insert_actor_assigns_id() {
        let db = Db::open_in_memory().unwrap();
        let hash = insert_blob_if_absent(db.conn(), b"some entity body").unwrap();
        let id = insert_actor(
            db.conn(),
            "Persistent.Foo_42",
            "/Game/Foo.Foo_C",
            "Level Persistent_Level",
            None,
            hash,
        )
        .unwrap();
        assert!(id > 0);
        let id2 = insert_actor(
            db.conn(),
            "Persistent.Foo_42",
            "/Game/Foo.Foo_C",
            "Level Persistent_Level",
            None,
            hash,
        )
        .unwrap();
        assert_ne!(id, id2);
    }
}
