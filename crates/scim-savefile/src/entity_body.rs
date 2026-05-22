//! `EntityBody` — the result of parsing a `RawActor`'s entity body bytes.
//!
//! Layout per JS Read.js:600-682:
//! - For entities whose object header is `ObjectKind::Actor`:
//!   - Entity reference: `ObjectProperty` (2 strings)
//!   - Child count: i32
//!   - Children: `ObjectProperty × child_count`
//!   - THEN the property bag (or end-of-body if exactly consumed)
//! - For entities whose object header is `ObjectKind::Object`:
//!   - Directly the property bag.
//!
//! The property bag walks until the `"None"` sentinel OR an unsupported type
//! (P1.3-a only handles primitives — composites surface via `first_unsupported`).
//! Any trailing bytes after the bag (e.g. `ConveyorBelt` extras) are captured as
//! `trailing_bytes` and reported but not parsed in P1.3-a.

use crate::error::Result;
use crate::object_header::ObjectKind;
use crate::object_property::{read_object_property, ObjectProperty};
use crate::property::{read_properties, Property, UnsupportedHit};
use crate::raw_actor::RawActor;
use crate::reader::Reader;

#[derive(Debug)]
pub struct EntityBody<'a> {
    /// Present only for `ObjectKind::Actor` entities. The actor's own reference.
    pub entity_reference: Option<ObjectProperty>,
    /// Child references (only populated for Actors).
    pub children: Vec<ObjectProperty>,
    /// Decoded properties (primitives only in P1.3-a).
    pub properties: Vec<Property>,
    /// Set when the property iterator stopped on an unsupported type.
    pub first_unsupported: Option<UnsupportedHit>,
    /// Bytes remaining in the entity body after the property bag.
    /// For ConveyorBelt/ConveyorChainActor these are extra unparsed bytes
    /// (deferred to a later phase).
    pub trailing_bytes: &'a [u8],
}

/// Parse the entity body of a single `RawActor`.
///
/// `save_version` is the LEVEL's `save_version` (`RawActor` doesn't carry it directly
/// — pass `LevelInfo::save_version` from the envelope).
/// `ue5_version`: 1000 for `save_version` < 53 (the only path P1.3-a supports).
/// `map_name`: the save's `Header::map_name`, needed for `ObjectProperty` collapsed-form.
pub fn parse_entity_body<'a>(
    raw_actor: &'a RawActor<'a>,
    save_version: i32,
    ue5_version: u32,
    map_name: &str,
) -> Result<EntityBody<'a>> {
    let body = raw_actor.entity.body_bytes;
    let mut r = Reader::new(body);

    let (entity_reference, children) = if raw_actor.header.kind == ObjectKind::Actor {
        let entity_reference = read_object_property(&mut r, map_name)?;
        let child_count = r.read_i32()?;
        let child_count_usize = usize::try_from(child_count.max(0)).unwrap_or(0);
        let mut children = Vec::with_capacity(child_count_usize);
        for _ in 0..child_count_usize {
            children.push(read_object_property(&mut r, map_name)?);
        }
        (Some(entity_reference), children)
    } else {
        (None, Vec::new())
    };

    // If preamble exactly filled the body, there's no property bag.
    if r.remaining() == 0 {
        return Ok(EntityBody {
            entity_reference,
            children,
            properties: Vec::new(),
            first_unsupported: None,
            trailing_bytes: &body[r.position()..],
        });
    }

    let parent_type = Some(raw_actor.header.class_name.as_str());
    let bag = read_properties(&mut r, save_version, ue5_version, map_name, parent_type)?;

    let trailing_bytes = &body[r.position()..];

    Ok(EntityBody {
        entity_reference,
        children,
        properties: bag.properties,
        first_unsupported: bag.first_unsupported,
        trailing_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::RawEntity;
    use crate::object_header::{ObjectHeaderBody, ObjectKind, RawObjectHeader, Transform};

    fn write_ascii(out: &mut Vec<u8>, s: &str) {
        let bytes = s.as_bytes();
        let len = i32::try_from(bytes.len() + 1).expect("string length fits i32");
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(bytes);
        out.push(0);
    }

    fn synth_actor_header() -> RawObjectHeader {
        RawObjectHeader {
            kind: ObjectKind::Actor,
            class_name: "/Game/Foo.Foo_C".to_string(),
            reference: crate::object_property::ObjectProperty {
                level_name: None,
                path_name: "Persistent.Foo_1".to_string(),
            },
            object_flags: None,
            body: ObjectHeaderBody::Actor {
                transform: Transform {
                    need_transform: 0,
                    rotation: [0.0; 4],
                    translation: [0.0; 3],
                    scale3d: [1.0; 3],
                    was_placed_in_level: 0,
                },
            },
        }
    }

    fn synth_object_header() -> RawObjectHeader {
        RawObjectHeader {
            kind: ObjectKind::Object,
            class_name: "/Game/Bar.Bar_C".to_string(),
            reference: crate::object_property::ObjectProperty {
                level_name: None,
                path_name: "Persistent.Bar_1".to_string(),
            },
            object_flags: None,
            body: ObjectHeaderBody::Object {
                outer_path_name: "Persistent.Outer".to_string(),
            },
        }
    }

    #[test]
    fn actor_with_empty_preamble_and_no_properties() {
        // entity_reference (expanded form, level != map_name) + child_count=0 + None.
        let mut body = Vec::new();
        write_ascii(&mut body, "Persistent_Level");
        write_ascii(&mut body, "Persistent.Foo_1");
        body.extend_from_slice(&0_i32.to_le_bytes()); // child_count
        write_ascii(&mut body, "None");

        let raw_actor = RawActor {
            level_name: "Level MapName",
            header: synth_actor_header(),
            entity: RawEntity {
                entity_save_version: 46,
                should_migrate_flag: 0,
                body_bytes: &body,
            },
        };
        let eb = parse_entity_body(&raw_actor, 46, 1000, "MapName").unwrap();
        assert!(eb.entity_reference.is_some());
        assert!(eb.children.is_empty());
        assert!(eb.properties.is_empty());
        assert!(eb.first_unsupported.is_none());
        assert_eq!(eb.trailing_bytes.len(), 0);
    }

    #[test]
    fn object_kind_skips_preamble() {
        // For ObjectKind::Object, the body starts directly with the property bag.
        let mut body = Vec::new();
        write_ascii(&mut body, "None"); // empty bag
        let raw_actor = RawActor {
            level_name: "Level MapName",
            header: synth_object_header(),
            entity: RawEntity {
                entity_save_version: 46,
                should_migrate_flag: 0,
                body_bytes: &body,
            },
        };
        let eb = parse_entity_body(&raw_actor, 46, 1000, "MapName").unwrap();
        assert!(eb.entity_reference.is_none());
        assert!(eb.children.is_empty());
        assert!(eb.properties.is_empty());
    }

    #[test]
    fn preamble_exactly_fills_body_produces_no_property_bag() {
        // 2 strings (entity ref) + 4 bytes (child_count=0) + NO bag, NO None sentinel.
        let mut body = Vec::new();
        write_ascii(&mut body, "Persistent_Level");
        write_ascii(&mut body, "Persistent.Foo_1");
        body.extend_from_slice(&0_i32.to_le_bytes());

        let raw_actor = RawActor {
            level_name: "Level MapName",
            header: synth_actor_header(),
            entity: RawEntity {
                entity_save_version: 46,
                should_migrate_flag: 0,
                body_bytes: &body,
            },
        };
        let eb = parse_entity_body(&raw_actor, 46, 1000, "MapName").unwrap();
        assert!(eb.entity_reference.is_some());
        assert!(eb.properties.is_empty());
        assert!(eb.first_unsupported.is_none());
    }
}
