//! Save-file header parsing.
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:30-69.

use crate::error::{Error, Result};
use crate::reader::Reader;
use crate::versions::{
    HAS_EDITOR_OBJECT_VERSION_FROM, HAS_MOD_METADATA_FROM, HAS_PARTITIONED_WORLD_FROM,
    HAS_SAVE_IDENTIFIER_FROM, HAS_SAVE_NAME_FROM, MAX_KNOWN_HEADER_TYPE, MIN_SUPPORTED_HEADER_TYPE,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub save_header_type: i32,
    pub save_version: i32,
    pub build_version: i32,
    pub save_name: Option<String>,
    pub map_name: String,
    pub map_options: String,
    pub session_name: String,
    pub play_duration_seconds: i32,
    pub save_date_time: i64,
    pub session_visibility: u8,
    pub editor_object_version: Option<i32>,
    pub mod_metadata: Option<String>,
    pub is_modded_save: Option<i32>,
    pub save_identifier: Option<String>,
    pub is_partitioned_world: Option<i32>,
    pub save_data_hash: Option<Vec<u8>>,
    pub is_creative_mode_enabled: Option<i32>,
}

/// Parse the header. Returns the header and the number of bytes consumed,
/// so the caller knows where the compressed body begins.
pub fn read_header(bytes: &[u8]) -> Result<(Header, usize)> {
    let mut r = Reader::new(bytes);

    let save_header_type = r.read_i32()?;
    if !(MIN_SUPPORTED_HEADER_TYPE..=MAX_KNOWN_HEADER_TYPE).contains(&save_header_type) {
        return Err(Error::UnsupportedHeaderType {
            found: save_header_type,
        });
    }

    let save_version = r.read_i32()?;
    let build_version = r.read_i32()?;

    let save_name = if save_header_type >= HAS_SAVE_NAME_FROM {
        Some(r.read_string()?)
    } else {
        None
    };

    let map_name = r.read_string()?;
    let map_options = r.read_string()?;
    let session_name = r.read_string()?;
    let play_duration_seconds = r.read_i32()?;
    let save_date_time = r.read_i64()?;
    let session_visibility = r.read_u8()?;

    let editor_object_version = if save_header_type >= HAS_EDITOR_OBJECT_VERSION_FROM {
        Some(r.read_i32()?)
    } else {
        None
    };

    let (mod_metadata, is_modded_save) = if save_header_type >= HAS_MOD_METADATA_FROM {
        (Some(r.read_string()?), Some(r.read_i32()?))
    } else {
        (None, None)
    };

    let save_identifier = if save_header_type >= HAS_SAVE_IDENTIFIER_FROM {
        Some(r.read_string()?)
    } else {
        None
    };

    let (is_partitioned_world, save_data_hash, is_creative_mode_enabled) =
        if save_header_type >= HAS_PARTITIONED_WORLD_FROM {
            (
                Some(r.read_i32()?),
                Some(r.read_hex(20)?),
                Some(r.read_i32()?),
            )
        } else {
            (None, None, None)
        };

    let header = Header {
        save_header_type,
        save_version,
        build_version,
        save_name,
        map_name,
        map_options,
        session_name,
        play_duration_seconds,
        save_date_time,
        session_visibility,
        editor_object_version,
        mod_metadata,
        is_modded_save,
        save_identifier,
        is_partitioned_world,
        save_data_hash,
        is_creative_mode_enabled,
    };

    Ok((header, r.position()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic save header (type 13) with known field values.
    /// This is the format used by recent game versions; covers most code paths.
    fn synth_header_type_13() -> Vec<u8> {
        let mut b = Vec::new();
        // save_header_type
        b.extend_from_slice(&13_i32.to_le_bytes());
        // save_version
        b.extend_from_slice(&41_i32.to_le_bytes());
        // build_version
        b.extend_from_slice(&368_883_i32.to_le_bytes());
        // map_name (header_type 13 does NOT include save_name; that's >=14)
        write_ascii(&mut b, "Persistent_Level");
        // map_options
        write_ascii(&mut b, "?listen");
        // session_name
        write_ascii(&mut b, "My Test Save");
        // play_duration_seconds
        b.extend_from_slice(&1234_i32.to_le_bytes());
        // save_date_time (UE ticks)
        b.extend_from_slice(&637_000_000_000_000_000_i64.to_le_bytes());
        // session_visibility
        b.push(0);
        // editor_object_version (>=7)
        b.extend_from_slice(&50_i32.to_le_bytes());
        // mod_metadata (>=8)
        write_ascii(&mut b, "");
        // is_modded_save (>=8)
        b.extend_from_slice(&0_i32.to_le_bytes());
        // save_identifier (>=10)
        write_ascii(&mut b, "ABC123");
        // is_partitioned_world (>=13)
        b.extend_from_slice(&1_i32.to_le_bytes());
        // save_data_hash (>=13) — 20 bytes
        b.extend_from_slice(&[0xAA; 20]);
        // is_creative_mode_enabled (>=13)
        b.extend_from_slice(&1_i32.to_le_bytes());
        b
    }

    fn write_ascii(out: &mut Vec<u8>, s: &str) {
        let bytes = s.as_bytes();
        let len = i32::try_from(bytes.len() + 1).unwrap(); // +1 for null terminator
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(bytes);
        out.push(0); // null terminator
    }

    #[test]
    fn read_header_type_13_returns_all_expected_fields() {
        let bytes = synth_header_type_13();
        let (h, consumed) = read_header(&bytes).unwrap();
        assert_eq!(h.save_header_type, 13);
        assert_eq!(h.save_version, 41);
        assert_eq!(h.build_version, 368_883);
        assert_eq!(
            h.save_name, None,
            "save_name appears only at header_type >= 14"
        );
        assert_eq!(h.map_name, "Persistent_Level");
        assert_eq!(h.map_options, "?listen");
        assert_eq!(h.session_name, "My Test Save");
        assert_eq!(h.play_duration_seconds, 1234);
        assert_eq!(h.save_date_time, 637_000_000_000_000_000);
        assert_eq!(h.session_visibility, 0);
        assert_eq!(h.editor_object_version, Some(50));
        assert_eq!(h.mod_metadata.as_deref(), Some(""));
        assert_eq!(h.is_modded_save, Some(0));
        assert_eq!(h.save_identifier.as_deref(), Some("ABC123"));
        assert_eq!(h.is_partitioned_world, Some(1));
        assert_eq!(h.save_data_hash.as_deref(), Some(&[0xAA; 20][..]));
        assert_eq!(h.is_creative_mode_enabled, Some(1));
        assert_eq!(consumed, bytes.len());
    }

    fn synth_header_type_7() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&7_i32.to_le_bytes());
        b.extend_from_slice(&10_i32.to_le_bytes());
        b.extend_from_slice(&12345_i32.to_le_bytes());
        write_ascii(&mut b, "Persistent_Level");
        write_ascii(&mut b, "");
        write_ascii(&mut b, "Old Save");
        b.extend_from_slice(&0_i32.to_le_bytes());
        b.extend_from_slice(&0_i64.to_le_bytes());
        b.push(0);
        b.extend_from_slice(&7_i32.to_le_bytes()); // editor_object_version (>=7)
        b
    }

    #[test]
    fn read_header_type_7_omits_optional_fields() {
        let bytes = synth_header_type_7();
        let (h, _) = read_header(&bytes).unwrap();
        assert_eq!(h.save_header_type, 7);
        assert_eq!(h.save_name, None);
        assert_eq!(h.editor_object_version, Some(7));
        assert_eq!(
            h.mod_metadata, None,
            "mod_metadata appears only at header_type >= 8"
        );
        assert_eq!(h.save_identifier, None);
        assert_eq!(h.is_partitioned_world, None);
    }

    #[test]
    fn read_header_rejects_oversize_type() {
        let bytes = 100_i32.to_le_bytes();
        let err = read_header(&bytes).unwrap_err();
        assert!(matches!(err, Error::UnsupportedHeaderType { found: 100 }));
    }

    #[test]
    fn read_header_rejects_negative_type() {
        let bytes = (-1_i32).to_le_bytes();
        let err = read_header(&bytes).unwrap_err();
        assert!(matches!(err, Error::UnsupportedHeaderType { found: -1 }));
    }

    fn synth_header_type_14() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&14_i32.to_le_bytes());
        b.extend_from_slice(&41_i32.to_le_bytes());
        b.extend_from_slice(&368_883_i32.to_le_bytes());
        // save_name appears for the first time at header_type >= 14
        write_ascii(&mut b, "MySave");
        write_ascii(&mut b, "Persistent_Level");
        write_ascii(&mut b, "?listen");
        write_ascii(&mut b, "Session Name");
        b.extend_from_slice(&500_i32.to_le_bytes());
        b.extend_from_slice(&637_000_000_000_000_000_i64.to_le_bytes());
        b.push(1);
        b.extend_from_slice(&50_i32.to_le_bytes());
        write_ascii(&mut b, "");
        b.extend_from_slice(&0_i32.to_le_bytes());
        write_ascii(&mut b, "SAVEID");
        b.extend_from_slice(&0_i32.to_le_bytes());
        b.extend_from_slice(&[0xBB; 20]);
        b.extend_from_slice(&0_i32.to_le_bytes());
        b
    }

    #[test]
    fn read_header_type_14_includes_save_name() {
        let bytes = synth_header_type_14();
        let (h, consumed) = read_header(&bytes).unwrap();
        assert_eq!(h.save_header_type, 14);
        assert_eq!(
            h.save_name.as_deref(),
            Some("MySave"),
            "save_name appears only at header_type >= 14"
        );
        assert_eq!(h.map_name, "Persistent_Level");
        assert_eq!(h.session_name, "Session Name");
        assert_eq!(consumed, bytes.len());
    }

    #[test]
    fn read_header_rejects_type_just_above_max() {
        // 15 is just past MAX_KNOWN_HEADER_TYPE (14). This is the value users will see
        // first when a new game patch ships before we update versions.rs.
        let bytes = 15_i32.to_le_bytes();
        let err = read_header(&bytes).unwrap_err();
        assert!(matches!(err, Error::UnsupportedHeaderType { found: 15 }));
    }

    #[test]
    fn read_header_rejects_type_below_min() {
        // 6 is just below MIN_SUPPORTED_HEADER_TYPE (7). Pre-Update-3 era; not supported.
        let bytes = 6_i32.to_le_bytes();
        let err = read_header(&bytes).unwrap_err();
        assert!(matches!(err, Error::UnsupportedHeaderType { found: 6 }));
    }
}
