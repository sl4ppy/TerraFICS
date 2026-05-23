//! Content-addressed blob storage.
//!
//! Each blob is hashed (`blake3`) on the DECOMPRESSED bytes; stored bytes are
//! zstd-compressed for space.
//!
//! Cross-reference: design spec §5.2.

use std::fmt::Write as _;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// 32-byte `blake3` hash of an actor's decompressed entity-body bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlobHash([u8; 32]);

impl BlobHash {
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let hash = blake3::hash(bytes);
        Self(*hash.as_bytes())
    }

    #[must_use]
    pub const fn from_array(arr: [u8; 32]) -> Self {
        Self(arr)
    }

    #[must_use]
    pub const fn as_array(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in &self.0 {
            let _ = write!(s, "{b:02x}");
        }
        s
    }
}

fn compress(bytes: &[u8]) -> Result<Vec<u8>> {
    zstd::encode_all(bytes, 3).map_err(Error::from)
}

fn decompress(zstd_bytes: &[u8]) -> Result<Vec<u8>> {
    zstd::decode_all(zstd_bytes).map_err(Error::from)
}

/// Insert a blob, deduplicating on the hash. If a row with this hash already
/// exists, the existing zstd data is untouched.
pub fn insert_blob_if_absent(conn: &Connection, bytes: &[u8]) -> Result<BlobHash> {
    let hash = BlobHash::from_bytes(bytes);
    let compressed = compress(bytes)?;
    conn.execute(
        "INSERT OR IGNORE INTO blob(hash, zstd_data) VALUES (?, ?)",
        params![hash.0.as_slice(), compressed.as_slice()],
    )?;
    Ok(hash)
}

/// Read the decompressed bytes for a blob. Verifies the hash matches.
pub fn read_blob(conn: &Connection, hash: BlobHash) -> Result<Option<Vec<u8>>> {
    let row: Option<Vec<u8>> = conn
        .query_row(
            "SELECT zstd_data FROM blob WHERE hash = ?",
            params![hash.0.as_slice()],
            |r| r.get(0),
        )
        .optional()?;
    let Some(zstd_data) = row else {
        return Ok(None);
    };
    let plain = decompress(&zstd_data)?;
    let actual = BlobHash::from_bytes(&plain);
    if actual != hash {
        return Err(Error::BlobHashMismatch {
            hex_hash: hash.to_hex(),
            actual_hex: actual.to_hex(),
        });
    }
    Ok(Some(plain))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    #[test]
    fn hash_is_deterministic() {
        let h1 = BlobHash::from_bytes(b"hello, world");
        let h2 = BlobHash::from_bytes(b"hello, world");
        assert_eq!(h1, h2);
        let h3 = BlobHash::from_bytes(b"hello, World");
        assert_ne!(h1, h3);
    }

    #[test]
    fn insert_and_read_roundtrip() {
        let db = Db::open_in_memory().unwrap();
        let payload = b"actor entity body bytes go here";
        let hash = insert_blob_if_absent(db.conn(), payload).unwrap();
        let read = read_blob(db.conn(), hash).unwrap().unwrap();
        assert_eq!(read.as_slice(), payload);
    }

    #[test]
    fn insert_is_idempotent() {
        let db = Db::open_in_memory().unwrap();
        let payload = b"same bytes twice";
        let h1 = insert_blob_if_absent(db.conn(), payload).unwrap();
        let h2 = insert_blob_if_absent(db.conn(), payload).unwrap();
        assert_eq!(h1, h2);
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM blob", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn read_missing_returns_none() {
        let db = Db::open_in_memory().unwrap();
        let h = BlobHash::from_bytes(b"unwritten");
        assert!(read_blob(db.conn(), h).unwrap().is_none());
    }
}
