//! Pure parser/writer for Satisfactory `.sav` files.
//! No I/O of its own — callers pass byte slices.
//!
//! P1.1: header parsing only. Full actor streaming lands in P1.2.

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
