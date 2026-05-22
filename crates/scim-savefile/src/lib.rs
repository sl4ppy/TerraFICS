//! Pure parser/writer for Satisfactory `.sav` files.
//! No I/O of its own — callers pass byte slices.
//!
//! Current capability:
//! - `read_header`: parse the save-file header (P1.1)
//! - `read_body`: decompress the zlib-compressed body chunks (P1.2-a)
//! - `read_body_envelope`: walk the body's outer structure to locate per-level
//!   objects/entities byte ranges (P1.2-b1)
//!
//! Roadmap: P1.2-b2 adds per-level object/actor parsing as a `RawActor` stream.

// modules added in later tasks
pub mod error;
pub use error::{Error, Result};
pub mod header;
pub mod reader;
pub mod versions;
pub use header::{read_header, Header};
pub mod body;
pub mod chunk_header;
pub use body::read_body;
pub mod body_envelope;
pub use body_envelope::{
    read_body_envelope, BodyEnvelope, LevelInfo, PartitionData, PartitionLevel, Partitions,
};
