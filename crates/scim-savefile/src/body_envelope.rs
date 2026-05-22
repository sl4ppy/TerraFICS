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
    // We rely on read_body to have already rejected save_version < 41.
    // 53+ adds a DataPackageVersion blob we haven't implemented yet.
    if header.save_version >= 53 {
        return Err(Error::UnsupportedSaveVersion {
            found: header.save_version,
        });
    }

    let mut r = Reader::new(body_bytes);

    // Skip the leading total-inflated-length prefix (u64 for >= 41).
    let _total_inflated_length = r.read_i64()?;

    // Partitions + levels come in subsequent tasks.
    Ok(BodyEnvelope {
        partitions: None,
        levels: Vec::new(),
        body_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal `Header` for tests. Only the fields `read_body_envelope` inspects
    /// are populated; everything else is a placeholder.
    fn synth_header(save_version: i32, is_partitioned: bool, map_name: &str) -> Header {
        Header {
            save_header_type: 14,
            save_version,
            build_version: 0,
            save_name: None,
            map_name: map_name.to_string(),
            map_options: String::new(),
            session_name: String::new(),
            play_duration_seconds: 0,
            save_date_time: 0,
            session_visibility: 0,
            editor_object_version: None,
            mod_metadata: None,
            is_modded_save: None,
            save_identifier: None,
            is_partitioned_world: Some(i32::from(is_partitioned)),
            save_data_hash: None,
            is_creative_mode_enabled: None,
        }
    }

    #[allow(dead_code)]
    fn write_ascii(out: &mut Vec<u8>, s: &str) {
        let bytes = s.as_bytes();
        let len = i32::try_from(bytes.len() + 1).expect("string length fits i32");
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(bytes);
        out.push(0);
    }

    #[test]
    fn rejects_save_version_53_and_above() {
        let header = synth_header(53, false, "Persistent_Level");
        let body = vec![0u8; 8];
        let err = read_body_envelope(&body, &header).unwrap_err();
        assert!(matches!(err, Error::UnsupportedSaveVersion { found: 53 }));
    }

    #[test]
    fn consumes_length_prefix() {
        // Body with: length prefix (8 bytes) + 4 bytes placeholder.
        // The placeholder gets read as nb_levels in a later task; for now
        // the function returns Ok with an empty levels vec because we haven't
        // implemented the level walk yet.
        let mut body = Vec::new();
        body.extend_from_slice(&0_u64.to_le_bytes()); // length prefix
        body.extend_from_slice(&0_i32.to_le_bytes()); // (will be nb_levels later)
        let header = synth_header(46, false, "Persistent_Level");
        let env = read_body_envelope(&body, &header).unwrap();
        assert!(env.partitions.is_none());
        assert!(env.levels.is_empty());
    }
}
