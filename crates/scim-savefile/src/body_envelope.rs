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

fn read_partitions(r: &mut Reader<'_>) -> Result<Partitions> {
    let partition_count = r.read_i32()?;
    r.read_string()?; // skip None marker
    r.read_u32()?;    // skip zero uint
    let head_hex_1 = r.read_u32()?;
    r.read_i32()?;    // skip one int
    r.read_string()?; // skip None marker
    let head_hex_2 = r.read_u32()?;

    let mut data = Vec::new();
    // The JS source iterates `i = 1; i < partition_count`, i.e. partition_count - 1 entries.
    for _ in 1..partition_count {
        let name = r.read_string()?;
        let grid_hex = r.read_u32()?;
        let count = r.read_u32()?;
        let nb_levels = r.read_i32()?;

        let mut levels = Vec::with_capacity(usize::try_from(nb_levels.max(0)).unwrap_or(0));
        for _ in 0..nb_levels {
            let level_name = r.read_string()?;
            let level_hex = r.read_u32()?;
            levels.push(PartitionLevel { name: level_name, level_hex });
        }

        data.push(PartitionData { name, grid_hex, count, levels });
    }

    Ok(Partitions { head_hex_1, head_hex_2, data })
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

    let partitions = if header.save_version >= 41 && header.is_partitioned_world == Some(1) {
        Some(read_partitions(&mut r)?)
    } else {
        None
    };

    // Levels come in the next task.
    Ok(BodyEnvelope {
        partitions,
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

    #[test]
    fn no_partitions_when_header_says_not_partitioned() {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u64.to_le_bytes()); // length prefix
        body.extend_from_slice(&0_i32.to_le_bytes()); // nb_levels (parsed in next task)
        let header = synth_header(46, /* is_partitioned */ false, "Persistent_Level");
        let env = read_body_envelope(&body, &header).unwrap();
        assert!(env.partitions.is_none());
    }

    /// Build a minimal valid partitions block with 1 partition entry (`partition_count` = 2,
    /// so the loop runs once for `i = 1`).
    fn synth_partitions_block() -> Vec<u8> {
        let mut b = Vec::new();
        // partition_count = 2
        b.extend_from_slice(&2_i32.to_le_bytes());
        write_ascii(&mut b, "None");                  // skipped string
        b.extend_from_slice(&0_u32.to_le_bytes());    // skipped uint = 0
        b.extend_from_slice(&0xCAFE_BABE_u32.to_le_bytes()); // head_hex_1
        b.extend_from_slice(&1_i32.to_le_bytes());    // skipped int = 1
        write_ascii(&mut b, "None");                  // skipped string
        b.extend_from_slice(&0xDEAD_BEEF_u32.to_le_bytes()); // head_hex_2

        // One partition data block (partition_count - 1 = 1 iteration)
        write_ascii(&mut b, "MainGrid");
        b.extend_from_slice(&0x1111_2222_u32.to_le_bytes()); // grid_hex
        b.extend_from_slice(&5_u32.to_le_bytes());           // count
        b.extend_from_slice(&2_i32.to_le_bytes());           // nb_levels = 2

        // 2 sub-level entries
        write_ascii(&mut b, "SubLevel1");
        b.extend_from_slice(&0x3333_4444_u32.to_le_bytes()); // level_hex
        write_ascii(&mut b, "SubLevel2");
        b.extend_from_slice(&0x5555_6666_u32.to_le_bytes());

        b
    }

    #[test]
    fn parses_partitions_block() {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u64.to_le_bytes()); // length prefix
        body.extend_from_slice(&synth_partitions_block());
        body.extend_from_slice(&0_i32.to_le_bytes()); // nb_levels = 0 (no levels follow)
        let header = synth_header(46, /* is_partitioned */ true, "Persistent_Level");
        let env = read_body_envelope(&body, &header).unwrap();
        let p = env.partitions.expect("expected partitions");
        assert_eq!(p.head_hex_1, 0xCAFE_BABE);
        assert_eq!(p.head_hex_2, 0xDEAD_BEEF);
        assert_eq!(p.data.len(), 1);
        let pd = &p.data[0];
        assert_eq!(pd.name, "MainGrid");
        assert_eq!(pd.grid_hex, 0x1111_2222);
        assert_eq!(pd.count, 5);
        assert_eq!(pd.levels.len(), 2);
        assert_eq!(pd.levels[0].name, "SubLevel1");
        assert_eq!(pd.levels[0].level_hex, 0x3333_4444);
        assert_eq!(pd.levels[1].name, "SubLevel2");
        assert_eq!(pd.levels[1].level_hex, 0x5555_6666);
    }
}
