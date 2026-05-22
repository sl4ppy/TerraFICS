//! Per-object metadata at the start of each level's objects block.
//!
//! Each record begins with an i32 type discriminator: 0 = Object, 1 = Actor.
//! Both share a leading (className, `ObjectProperty`, optional `object_flags`)
//! prefix; they diverge in their trailing fields.
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:529-598.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::object_property::{read_object_property, ObjectProperty};
use crate::reader::Reader;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectKind {
    Object,
    Actor,
}

/// Unreal-style transform: rotation (quat), translation (vec3), scale (vec3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub need_transform: i32,
    pub rotation: [f32; 4],
    pub translation: [f32; 3],
    pub scale3d: [f32; 3],
    pub was_placed_in_level: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectHeaderBody {
    Object { outer_path_name: String },
    Actor { transform: Transform },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawObjectHeader {
    pub kind: ObjectKind,
    pub class_name: String,
    pub reference: ObjectProperty,
    /// Present only when the owning level has `save_version >= 51`.
    pub object_flags: Option<u32>,
    pub body: ObjectHeaderBody,
}

/// Parse the next object-header record from `r`, given the owning level's `save_version`
/// and the save's `map_name` (needed for `ObjectProperty`'s collapsed-form decision).
pub fn read_object_header(
    r: &mut Reader<'_>,
    level_save_version: i32,
    map_name: &str,
) -> Result<RawObjectHeader> {
    let object_type = r.read_i32()?;
    let kind = match object_type {
        0 => ObjectKind::Object,
        1 => ObjectKind::Actor,
        other => {
            return Err(Error::UnsupportedObjectType { found: other });
        }
    };

    let class_name = r.read_string()?;
    let reference = read_object_property(r, map_name)?;

    let object_flags = if level_save_version >= 51 {
        Some(r.read_u32()?)
    } else {
        None
    };

    let body = match kind {
        ObjectKind::Object => {
            let outer_path_name = r.read_string()?;
            ObjectHeaderBody::Object { outer_path_name }
        }
        ObjectKind::Actor => {
            let need_transform = r.read_i32()?;
            let rotation = [r.read_f32()?, r.read_f32()?, r.read_f32()?, r.read_f32()?];
            let translation = [r.read_f32()?, r.read_f32()?, r.read_f32()?];
            let scale3d = [r.read_f32()?, r.read_f32()?, r.read_f32()?];
            let was_placed_in_level = r.read_i32()?;
            ObjectHeaderBody::Actor {
                transform: Transform {
                    need_transform,
                    rotation,
                    translation,
                    scale3d,
                    was_placed_in_level,
                },
            }
        }
    };

    Ok(RawObjectHeader {
        kind,
        class_name,
        reference,
        object_flags,
        body,
    })
}

/// Walk a level's objects block, parsing every object header.
///
/// `bytes` is the slice corresponding to `LevelInfo::objects_byte_range`.
/// Returns the list of headers. Any trailing bytes (level-persistent flag, collectables-
/// in-between, etc.) are silently ignored — the envelope already partitions them out.
pub fn read_objects_in_level(
    bytes: &[u8],
    level_save_version: i32,
    map_name: &str,
) -> Result<Vec<RawObjectHeader>> {
    let mut r = Reader::new(bytes);
    let count = r.read_i32()?;
    let count_usize = usize::try_from(count.max(0)).unwrap_or(0);

    let mut out = Vec::with_capacity(count_usize);
    for _ in 0..count_usize {
        out.push(read_object_header(&mut r, level_save_version, map_name)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_ascii(out: &mut Vec<u8>, s: &str) {
        let bytes = s.as_bytes();
        let len = i32::try_from(bytes.len() + 1).expect("string length fits i32");
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(bytes);
        out.push(0);
    }

    /// Build a synthetic Object (type=0) header.
    fn synth_object_v46() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&0_i32.to_le_bytes()); // type = 0 (Object)
        write_ascii(&mut b, "/Game/FactoryGame/Foo.Foo_C");
        // ObjectProperty: expanded form (level_name != map_name)
        write_ascii(&mut b, "Persistent_Level");
        write_ascii(&mut b, "Persistent.Foo_42");
        // no object_flags (save_version < 51)
        write_ascii(&mut b, "Persistent_Level:PersistentLevel.OuterFoo");
        b
    }

    #[test]
    fn parses_object_at_save_version_46() {
        let bytes = synth_object_v46();
        let mut r = Reader::new(&bytes);
        let h = read_object_header(&mut r, 46, "MapName").unwrap();
        assert_eq!(h.kind, ObjectKind::Object);
        assert_eq!(h.class_name, "/Game/FactoryGame/Foo.Foo_C");
        assert_eq!(h.reference.level_name.as_deref(), Some("Persistent_Level"));
        assert_eq!(h.reference.path_name, "Persistent.Foo_42");
        assert_eq!(h.object_flags, None);
        match h.body {
            ObjectHeaderBody::Object { outer_path_name } => {
                assert_eq!(outer_path_name, "Persistent_Level:PersistentLevel.OuterFoo");
            }
            ObjectHeaderBody::Actor { .. } => panic!("expected Object body"),
        }
    }

    /// Build a synthetic Actor (type=1) header at `save_version` 52 (`object_flags` present).
    fn synth_actor_v52() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&1_i32.to_le_bytes()); // type = 1 (Actor)
        write_ascii(&mut b, "/Game/FactoryGame/Build_Assembler.Build_Assembler_C");
        write_ascii(&mut b, "Persistent_Level");
        write_ascii(&mut b, "Persistent.Build_Assembler_C_42");
        b.extend_from_slice(&0x1234_5678_u32.to_le_bytes()); // object_flags
        b.extend_from_slice(&1_i32.to_le_bytes()); // need_transform
        // rotation
        b.extend_from_slice(&0.0_f32.to_le_bytes());
        b.extend_from_slice(&0.0_f32.to_le_bytes());
        b.extend_from_slice(&0.0_f32.to_le_bytes());
        b.extend_from_slice(&1.0_f32.to_le_bytes());
        // translation
        b.extend_from_slice(&100.0_f32.to_le_bytes());
        b.extend_from_slice(&200.0_f32.to_le_bytes());
        b.extend_from_slice(&50.0_f32.to_le_bytes());
        // scale3d
        b.extend_from_slice(&1.0_f32.to_le_bytes());
        b.extend_from_slice(&1.0_f32.to_le_bytes());
        b.extend_from_slice(&1.0_f32.to_le_bytes());
        // was_placed_in_level
        b.extend_from_slice(&0_i32.to_le_bytes());
        b
    }

    #[test]
    fn parses_actor_at_save_version_52() {
        let bytes = synth_actor_v52();
        let mut r = Reader::new(&bytes);
        let h = read_object_header(&mut r, 52, "MapName").unwrap();
        assert_eq!(h.kind, ObjectKind::Actor);
        assert_eq!(h.object_flags, Some(0x1234_5678_u32));
        match h.body {
            ObjectHeaderBody::Actor { transform } => {
                assert_eq!(transform.need_transform, 1);
                assert!(transform.rotation.iter().zip([0.0, 0.0, 0.0, 1.0]).all(|(a, b)| (a - b).abs() < f32::EPSILON));
                assert!(transform.translation.iter().zip([100.0, 200.0, 50.0]).all(|(a, b)| (a - b).abs() < f32::EPSILON));
                assert!(transform.scale3d.iter().zip([1.0, 1.0, 1.0]).all(|(a, b)| (a - b).abs() < f32::EPSILON));
                assert_eq!(transform.was_placed_in_level, 0);
            }
            ObjectHeaderBody::Object { .. } => panic!("expected Actor body"),
        }
    }

    #[test]
    fn reads_objects_in_level_with_two_actors() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&2_i32.to_le_bytes()); // count = 2
        bytes.extend(synth_actor_v52());
        bytes.extend(synth_actor_v52());
        let headers = read_objects_in_level(&bytes, 52, "MapName").unwrap();
        assert_eq!(headers.len(), 2);
        for h in &headers {
            assert_eq!(h.kind, ObjectKind::Actor);
        }
    }

    #[test]
    fn reads_empty_objects_block() {
        let bytes = 0_i32.to_le_bytes(); // count = 0
        let headers = read_objects_in_level(&bytes, 46, "MapName").unwrap();
        assert!(headers.is_empty());
    }

    #[test]
    fn rejects_unknown_object_type() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&7_i32.to_le_bytes()); // bogus type
        let mut r = Reader::new(&bytes);
        let err = read_object_header(&mut r, 46, "MapName").unwrap_err();
        assert!(matches!(err, Error::UnsupportedObjectType { found: 7 }));
    }
}
