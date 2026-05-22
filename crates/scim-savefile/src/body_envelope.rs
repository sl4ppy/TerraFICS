//! Walks the outer structure of a decompressed `.sav` body to locate each level's
//! objects/entities byte ranges. Does not parse the contents of those ranges —
//! that's P1.2-b2's job.
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:200-525.

use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::header::Header;
use crate::reader::Reader;

/// Top-level descriptor of a decompressed save body.
/// Holds offsets into `body_bytes`; the caller still owns the underlying data.
#[derive(Debug)]
pub struct BodyEnvelope<'a> {
    pub partitions: Option<Partitions>,
    pub levels: Vec<LevelInfo>,
    pub body_bytes: &'a [u8],
}

/// One level (sub-level or the synthetic main level) inside the body.
/// `objects_byte_range` and `entities_byte_range` index into `BodyEnvelope::body_bytes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelInfo {
    pub name: String,
    /// Per-level `save_version`. For the main level this equals `Header.save_version`.
    /// For sub-levels with `header.save_version >= 51`, this is discovered via a
    /// forward scan past the level's entities block (see plan §"Leap of faith").
    pub save_version: i32,
    pub objects_byte_range: Range<usize>,
    pub entities_byte_range: Range<usize>,
}

/// World-partition metadata, present only when `header.is_partitioned_world == 1`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partitions {
    pub head_hex_1: u32,
    pub head_hex_2: u32,
    pub data: Vec<PartitionData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionData {
    pub name: String,
    pub grid_hex: u32,
    pub count: u32,
    pub levels: Vec<PartitionLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionLevel {
    pub name: String,
    pub level_hex: u32,
}

/// Parse the outer structure of a decompressed save body.
/// `body_bytes` is the buffer returned by `read_body`.
pub fn read_body_envelope<'a>(body_bytes: &'a [u8], header: &Header) -> Result<BodyEnvelope<'a>> {
    let _ = (body_bytes, header);
    let _: Reader<'_> = Reader::new(&[]);
    let _: Error = Error::UnsupportedSaveVersion { found: 0 };
    todo!("implement in subsequent tasks (4-6)")
}

#[cfg(test)]
mod tests {}
