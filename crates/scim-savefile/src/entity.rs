//! Per-entity preamble + length-prefixed property-bag body.
//!
//! For `save_version >= 41` (our minimum supported), each entity record is:
//!
//! | Offset | Size | Field |
//! |---|---|---|
//! | 0  | 4 | `entity_save_version` (u32) |
//! | 4  | 4 | `should_migrate_object_refs_to_persistent_flag` (u32) |
//! | 8  | 4 | `entity_length` (i32) — bytes in the body that follows |
//! | 12 | `entity_length` | body bytes (property bag — parsed by P1.3) |
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:600-630.

use crate::error::{Error, Result};
use crate::reader::Reader;

#[derive(Debug, Clone, Copy)]
pub struct RawEntity<'a> {
    pub entity_save_version: u32,
    pub should_migrate_flag: u32,
    pub body_bytes: &'a [u8],
}

/// Read one entity record from `r`. Advances `r` past the entire record (preamble + body).
pub fn read_entity<'a>(r: &mut Reader<'a>) -> Result<RawEntity<'a>> {
    let entity_save_version = r.read_u32()?;
    let should_migrate_flag = r.read_u32()?;
    let entity_length = r.read_i32()?;
    let entity_length_usize =
        usize::try_from(entity_length).map_err(|_| Error::ChunkLengthMismatch {
            at: r.position(),
            expected: u64::try_from(entity_length).unwrap_or(0),
            actual: 0,
        })?;

    let body_start = r.position();
    let body_end =
        body_start
            .checked_add(entity_length_usize)
            .ok_or_else(|| Error::UnexpectedEof {
                wanted: entity_length_usize,
                available: r.remaining(),
                at: body_start,
            })?;
    if body_end > r.position() + r.remaining() {
        return Err(Error::UnexpectedEof {
            wanted: body_end - r.position(),
            available: r.remaining(),
            at: r.position(),
        });
    }

    let body_bytes = &r.as_slice_from(body_start)[..entity_length_usize];
    r.seek(body_end);

    Ok(RawEntity {
        entity_save_version,
        should_migrate_flag,
        body_bytes,
    })
}

/// Walk a level's entities block.
///
/// `bytes` is the slice corresponding to `LevelInfo::entities_byte_range`.
pub fn read_entities_in_level(bytes: &[u8]) -> Result<Vec<RawEntity<'_>>> {
    let mut r = Reader::new(bytes);
    let count = r.read_i32()?;
    let count_usize = usize::try_from(count.max(0)).unwrap_or(0);
    let mut out = Vec::with_capacity(count_usize);
    for _ in 0..count_usize {
        out.push(read_entity(&mut r)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synth_entity(payload: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&46_u32.to_le_bytes()); // entity_save_version
        b.extend_from_slice(&0_u32.to_le_bytes()); // should_migrate
        b.extend_from_slice(&i32::try_from(payload.len()).unwrap().to_le_bytes());
        b.extend_from_slice(payload);
        b
    }

    #[test]
    fn reads_single_entity() {
        let payload = b"hello, property bag";
        let bytes = synth_entity(payload);
        let mut r = Reader::new(&bytes);
        let e = read_entity(&mut r).unwrap();
        assert_eq!(e.entity_save_version, 46);
        assert_eq!(e.should_migrate_flag, 0);
        assert_eq!(e.body_bytes, payload);
        assert_eq!(r.position(), bytes.len());
    }

    #[test]
    fn reads_entities_block_with_two_records() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&2_i32.to_le_bytes()); // count = 2
        bytes.extend(synth_entity(b"foo"));
        bytes.extend(synth_entity(b"bar baz"));
        let entities = read_entities_in_level(&bytes).unwrap();
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].body_bytes, b"foo");
        assert_eq!(entities[1].body_bytes, b"bar baz");
    }

    #[test]
    fn reads_empty_entities_block() {
        let bytes = 0_i32.to_le_bytes();
        let entities = read_entities_in_level(&bytes).unwrap();
        assert!(entities.is_empty());
    }

    #[test]
    fn truncated_entity_body_returns_eof() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&46_u32.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&1000_i32.to_le_bytes()); // claims 1000 body bytes
        bytes.extend_from_slice(&[0_u8; 5]); // but only 5 actually present
        let mut r = Reader::new(&bytes);
        let err = read_entity(&mut r).unwrap_err();
        assert!(matches!(err, Error::UnexpectedEof { wanted: 1000, .. }));
    }
}
