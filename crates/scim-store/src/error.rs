//! Concrete error types for `scim-store`.
//! No `anyhow` per design spec §11.1.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("savefile error: {0}")]
    Savefile(#[from] scim_savefile::Error),

    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("blob hash mismatch reading {hex_hash}: stored bytes hash to {actual_hex}")]
    BlobHashMismatch {
        hex_hash: String,
        actual_hex: String,
    },

    #[error("snapshot {snapshot_id} not found")]
    SnapshotNotFound { snapshot_id: i64 },

    #[error("actor {actor_id} not found")]
    ActorNotFound { actor_id: i64 },

    #[error("expected transform blob to be exactly 40 bytes, got {found}")]
    TransformBlobLength { found: usize },
}

#[allow(unused_attributes)]
#[must_use]
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn savefile_error_converts_via_from() {
        let underlying = scim_savefile::Error::UnexpectedEof {
            wanted: 4,
            available: 1,
            at: 0,
        };
        let wrapped: Error = underlying.into();
        assert!(wrapped.to_string().contains("savefile"));
    }
}
