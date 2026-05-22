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

/// Consume the `countCollected` i32 and any following collectable records.
/// Each collectable is two UE strings (levelName + pathName).
/// Cross-reference: Read.js:487-504 (post-entities block for non-main sub-levels).
fn skip_sub_level_collectables(r: &mut Reader<'_>) -> Result<()> {
    let count_collected = r.read_i32()?;
    for _ in 0..count_collected.max(0) {
        r.read_string()?; // levelName
        r.read_string()?; // pathName
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // Sequential parser with bounds checks, splitting would obscure flow
fn read_level_scaffold(
    r: &mut Reader<'_>,
    header: &Header,
    is_main_level: bool,
) -> Result<LevelInfo> {
    let name = if is_main_level {
        format!("Level {}", header.map_name)
    } else {
        r.read_string()?
    };

    // Binary lengths use i64 in save_version >= 41 (read_body already requires >= 41).
    let objects_binary_length = r.read_i64()?;
    let objects_binary_length_usize =
        usize::try_from(objects_binary_length).map_err(|_| Error::ChunkLengthMismatch {
            at: r.position(),
            expected: u64::try_from(objects_binary_length).unwrap_or(0),
            actual: 0,
        })?;
    let objects_start = r.position();

    // Per-level save_version: header default; sub-levels do a leap of faith for >= 51.
    let mut level_save_version = header.save_version;

    if header.save_version >= 51 && !is_main_level {
        // Forward scan: skip the objects block, read entities_binary_length, skip the
        // entities block, then read the trailing level_save_version (u32), then consume
        // the countCollected block (and, for >= 53, the data-package-version blob).
        // Restore r.position() to objects_start when done so sequential parsing continues.
        let entities_pos = objects_start
            .checked_add(objects_binary_length_usize)
            .ok_or_else(|| Error::UnexpectedEof {
                wanted: objects_binary_length_usize,
                available: r.remaining(),
                at: objects_start,
            })?;
        if entities_pos > r.position() + r.remaining() {
            return Err(Error::UnexpectedEof {
                wanted: entities_pos - r.position(),
                available: r.remaining(),
                at: r.position(),
            });
        }
        r.seek(entities_pos);
        let entities_binary_length = r.read_i64()?;
        let entities_binary_length_usize = usize::try_from(entities_binary_length)
            .map_err(|_| Error::ChunkLengthMismatch {
                at: r.position(),
                expected: u64::try_from(entities_binary_length).unwrap_or(0),
                actual: 0,
            })?;
        let after_entities_pos = r.position();
        let after_entities = after_entities_pos
            .checked_add(entities_binary_length_usize)
            .ok_or_else(|| Error::UnexpectedEof {
                wanted: entities_binary_length_usize,
                available: r.remaining(),
                at: after_entities_pos,
            })?;
        if after_entities > r.position() + r.remaining() {
            return Err(Error::UnexpectedEof {
                wanted: after_entities - r.position(),
                available: r.remaining(),
                at: r.position(),
            });
        }
        r.seek(after_entities);
        level_save_version = i32::try_from(r.read_u32()?).expect("save_version fits i32");
        // Consume the post-entities trailing block (countCollected + collectables).
        skip_sub_level_collectables(r)?;
        // Restore position so the caller continues at objects_start sequentially.
        r.seek(objects_start);
    }

    // Now walk the level sequentially.
    let objects_end = objects_start
        .checked_add(objects_binary_length_usize)
        .ok_or_else(|| Error::UnexpectedEof {
            wanted: objects_binary_length_usize,
            available: r.remaining(),
            at: objects_start,
        })?;
    if objects_end > r.position() + r.remaining() {
        return Err(Error::UnexpectedEof {
            wanted: objects_end - r.position(),
            available: r.remaining(),
            at: r.position(),
        });
    }
    r.seek(objects_end);

    let entities_binary_length = r.read_i64()?;
    let entities_binary_length_usize = usize::try_from(entities_binary_length)
        .map_err(|_| Error::ChunkLengthMismatch {
            at: r.position(),
            expected: u64::try_from(entities_binary_length).unwrap_or(0),
            actual: 0,
        })?;
    let entities_start = r.position();
    let entities_end = entities_start
        .checked_add(entities_binary_length_usize)
        .ok_or_else(|| Error::UnexpectedEof {
            wanted: entities_binary_length_usize,
            available: r.remaining(),
            at: entities_start,
        })?;
    if entities_end > r.position() + r.remaining() {
        return Err(Error::UnexpectedEof {
            wanted: entities_end - r.position(),
            available: r.remaining(),
            at: r.position(),
        });
    }
    r.seek(entities_end);

    if !is_main_level {
        // For sub-levels at save_version >= 51, the trailing per-level save_version was
        // already read during the leap of faith and is duplicated here; consume it.
        if header.save_version >= 51 {
            let _duplicate_save_version = r.read_u32()?;
        }
        // All sub-levels (any save_version) have a countCollected + collectables block
        // immediately after the entities block.  Each collectable is two UE strings
        // (levelName + pathName).  Cross-reference: Read.js:487-504.
        skip_sub_level_collectables(r)?;
    }

    Ok(LevelInfo {
        name,
        save_version: level_save_version,
        objects_byte_range: objects_start..objects_end,
        entities_byte_range: entities_start..entities_end,
    })
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

    let nb_levels = r.read_i32()?;
    let mut levels = Vec::with_capacity(usize::try_from(nb_levels.max(0)).unwrap_or(0) + 1);
    for _ in 0..nb_levels {
        levels.push(read_level_scaffold(&mut r, header, /* is_main_level */ false)?);
    }
    // The implicit main level always follows the explicit sub-levels.
    levels.push(read_level_scaffold(&mut r, header, /* is_main_level */ true)?);

    Ok(BodyEnvelope { partitions, levels, body_bytes })
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
        // Body with: length prefix (8 bytes) + nb_levels (4 bytes) + minimal main level.
        let mut body = Vec::new();
        body.extend_from_slice(&0_u64.to_le_bytes()); // length prefix
        body.extend_from_slice(&0_i32.to_le_bytes()); // nb_levels = 0
        // Minimal main level: objects_binary_length=0, entities_binary_length=0
        body.extend_from_slice(&0_i64.to_le_bytes()); // objects_binary_length
        body.extend_from_slice(&0_i64.to_le_bytes()); // entities_binary_length
        let header = synth_header(46, false, "Persistent_Level");
        let env = read_body_envelope(&body, &header).unwrap();
        assert!(env.partitions.is_none());
        assert_eq!(env.levels.len(), 1);
    }

    #[test]
    fn no_partitions_when_header_says_not_partitioned() {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u64.to_le_bytes()); // length prefix
        body.extend_from_slice(&0_i32.to_le_bytes()); // nb_levels = 0
        // Minimal main level: objects_binary_length=0, entities_binary_length=0
        body.extend_from_slice(&0_i64.to_le_bytes()); // objects_binary_length
        body.extend_from_slice(&0_i64.to_le_bytes()); // entities_binary_length
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

    /// Build a minimal body with `nb_levels = 0` (only the implicit main level)
    /// where the objects and entities blocks are arbitrary opaque bytes of given sizes.
    fn synth_body_one_main_level(
        save_version: i32,
        objects_block: &[u8],
        entities_block: &[u8],
    ) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&0_u64.to_le_bytes()); // length prefix
        b.extend_from_slice(&0_i32.to_le_bytes()); // nb_levels = 0; main level still follows

        // Main level: no name read (synthesized), objects_binary_length (i64), block,
        // entities_binary_length (i64), block.
        b.extend_from_slice(&i64::try_from(objects_block.len()).unwrap().to_le_bytes());
        b.extend_from_slice(objects_block);
        b.extend_from_slice(&i64::try_from(entities_block.len()).unwrap().to_le_bytes());
        b.extend_from_slice(entities_block);

        let _ = save_version; // currently unused; placeholder for future variants
        b
    }

    #[test]
    fn walks_one_main_level_with_correct_ranges() {
        let objects = vec![0xAA_u8; 30];
        let entities = vec![0xBB_u8; 70];
        let body = synth_body_one_main_level(46, &objects, &entities);
        let header = synth_header(46, false, "MyMap");
        let env = read_body_envelope(&body, &header).unwrap();
        assert_eq!(env.levels.len(), 1);
        let main = &env.levels[0];
        assert_eq!(main.name, "Level MyMap");
        assert_eq!(main.save_version, 46);
        // After 8-byte length prefix + 4-byte nb_levels + 8-byte objects_binary_length:
        let expected_objects_start = 8 + 4 + 8;
        let expected_objects_end = expected_objects_start + objects.len();
        assert_eq!(main.objects_byte_range, expected_objects_start..expected_objects_end);
        // After objects block + 8-byte entities_binary_length:
        let expected_entities_start = expected_objects_end + 8;
        let expected_entities_end = expected_entities_start + entities.len();
        assert_eq!(main.entities_byte_range, expected_entities_start..expected_entities_end);
    }

    #[test]
    fn parses_partitions_block() {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u64.to_le_bytes()); // length prefix
        body.extend_from_slice(&synth_partitions_block());
        body.extend_from_slice(&0_i32.to_le_bytes()); // nb_levels = 0
        // Minimal main level: objects_binary_length=0, entities_binary_length=0
        body.extend_from_slice(&0_i64.to_le_bytes()); // objects_binary_length
        body.extend_from_slice(&0_i64.to_le_bytes()); // entities_binary_length
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

    #[test]
    fn walks_two_sublevels_plus_main_at_save_version_46() {
        let objects_a = vec![0x10_u8; 4];
        let entities_a = vec![0x11_u8; 6];
        let objects_b = vec![0x20_u8; 8];
        let entities_b = vec![0x21_u8; 12];
        let objects_main = vec![0x30_u8; 5];
        let entities_main = vec![0x31_u8; 7];

        let mut body = Vec::new();
        body.extend_from_slice(&0_u64.to_le_bytes()); // length prefix
        body.extend_from_slice(&2_i32.to_le_bytes()); // nb_levels = 2

        // Sub-level A
        write_ascii(&mut body, "SubA");
        body.extend_from_slice(&i64::try_from(objects_a.len()).unwrap().to_le_bytes());
        body.extend_from_slice(&objects_a);
        body.extend_from_slice(&i64::try_from(entities_a.len()).unwrap().to_le_bytes());
        body.extend_from_slice(&entities_a);
        body.extend_from_slice(&0_i32.to_le_bytes()); // countCollected = 0

        // Sub-level B
        write_ascii(&mut body, "SubB");
        body.extend_from_slice(&i64::try_from(objects_b.len()).unwrap().to_le_bytes());
        body.extend_from_slice(&objects_b);
        body.extend_from_slice(&i64::try_from(entities_b.len()).unwrap().to_le_bytes());
        body.extend_from_slice(&entities_b);
        body.extend_from_slice(&0_i32.to_le_bytes()); // countCollected = 0

        // Main level (no name in stream)
        body.extend_from_slice(&i64::try_from(objects_main.len()).unwrap().to_le_bytes());
        body.extend_from_slice(&objects_main);
        body.extend_from_slice(&i64::try_from(entities_main.len()).unwrap().to_le_bytes());
        body.extend_from_slice(&entities_main);

        let header = synth_header(46, false, "MyMap");
        let env = read_body_envelope(&body, &header).unwrap();
        assert_eq!(env.levels.len(), 3);
        assert_eq!(env.levels[0].name, "SubA");
        assert_eq!(env.levels[0].save_version, 46);
        assert_eq!(env.levels[1].name, "SubB");
        assert_eq!(env.levels[2].name, "Level MyMap");
        // Sanity-check byte-range lengths
        assert_eq!(env.levels[0].objects_byte_range.len(), 4);
        assert_eq!(env.levels[0].entities_byte_range.len(), 6);
        assert_eq!(env.levels[1].objects_byte_range.len(), 8);
        assert_eq!(env.levels[1].entities_byte_range.len(), 12);
        assert_eq!(env.levels[2].objects_byte_range.len(), 5);
        assert_eq!(env.levels[2].entities_byte_range.len(), 7);
    }

    #[test]
    fn sub_level_save_version_discovered_via_leap_of_faith_at_save_version_52() {
        // For save_version >= 51, sub-levels have a trailing u32 with the per-sublevel
        // save_version. We use 49 (an arbitrary plausible value) here.
        let objects = vec![0xAA_u8; 4];
        let entities = vec![0xBB_u8; 6];
        let objects_main = vec![0x30_u8; 2];
        let entities_main = vec![0x31_u8; 2];

        let mut body = Vec::new();
        body.extend_from_slice(&0_u64.to_le_bytes()); // length prefix
        body.extend_from_slice(&1_i32.to_le_bytes()); // nb_levels = 1

        // Sub-level
        write_ascii(&mut body, "SubA");
        body.extend_from_slice(&i64::try_from(objects.len()).unwrap().to_le_bytes());
        body.extend_from_slice(&objects);
        body.extend_from_slice(&i64::try_from(entities.len()).unwrap().to_le_bytes());
        body.extend_from_slice(&entities);
        body.extend_from_slice(&49_u32.to_le_bytes()); // trailing per-sublevel save_version (>= 51)
        body.extend_from_slice(&0_i32.to_le_bytes()); // countCollected = 0

        // Main level
        body.extend_from_slice(&i64::try_from(objects_main.len()).unwrap().to_le_bytes());
        body.extend_from_slice(&objects_main);
        body.extend_from_slice(&i64::try_from(entities_main.len()).unwrap().to_le_bytes());
        body.extend_from_slice(&entities_main);

        let header = synth_header(52, false, "MyMap");
        let env = read_body_envelope(&body, &header).unwrap();
        assert_eq!(env.levels.len(), 2);
        assert_eq!(env.levels[0].name, "SubA");
        assert_eq!(env.levels[0].save_version, 49,
            "leap of faith should have discovered the sub-level's save_version");
        assert_eq!(env.levels[1].name, "Level MyMap");
        assert_eq!(env.levels[1].save_version, 52,
            "main level uses header.save_version directly");
    }
}
