//! SQLite-backed project store for parsed Satisfactory saves.
//!
//! P1.4 capability lands incrementally:
//! - Task 2: `Error` / `Result`
//! - Task 3: schema DDL
//! - Task 4: `SQLite` tuning
//! - Task 5: `Db` connection wrapper
//! - Task 6: blob storage (content-addressed via blake3 + zstd)
//! - Task 7: actor storage
//! - Task 8: snapshot creation
//! - Task 9: per-snapshot header JSON
//! - Task 10: `import_save` end-to-end
//! - Task 11: read API
//!
//! Roadmap: P2 adds the edit path (immutable actor rows mean new rows per edit).

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
