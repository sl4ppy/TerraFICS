//! `SQLite`-backed project store for parsed Satisfactory saves.
//!
//! Per design spec §5: one project DB per save being tracked, with content-
//! addressed blobs (`blake3` + `zstd`), immutable per-version actor rows, and
//! a snapshot DAG (P1.4 creates exactly one snapshot per import; P2 adds
//! per-edit child snapshots).
//!
//! Public API:
//! - `Db::open(path)` — open or create a project DB with tuning + schema
//!   applied.
//! - `import_save(db, sav_path, label)` — parse a `.sav` and write a fresh
//!   snapshot. Returns `ImportSummary`.
//! - `list_snapshots`, `list_actors_in_snapshot`, `read_blob` — read API.
//! - `header_store::insert_header`, `header_store::read_header` — per-snapshot
//!   `Header` JSON.
//!
//! Roadmap: P2 adds per-edit snapshots and the save-as-`.sav` write path.

pub mod error;
pub use error::{Error, Result};
pub mod schema;
pub use schema::apply_schema;
pub mod tuning;
pub use tuning::apply_tuning;
pub mod db;
pub use db::Db;
pub mod blob;
pub use blob::{insert_blob_if_absent, read_blob, BlobHash};
pub mod actor;
pub use actor::{
    decode_transform, encode_transform, insert_actor, list_actors_in_snapshot, ActorRow,
};
pub mod snapshot;
pub use snapshot::{add_actor_to_snapshot, create_snapshot, list_snapshots, SnapshotRow};
pub mod header_store;
pub mod import;
pub use import::{import_save, ImportSummary};
