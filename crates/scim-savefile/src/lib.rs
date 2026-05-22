//! Pure parser/writer for Satisfactory `.sav` files.
//! No I/O of its own — callers pass byte slices.
//!
//! Current capability:
//! - `read_header`: parse the save-file header (P1.1)
//! - `read_body`: decompress the zlib-compressed body chunks (P1.2-a)
//! - `read_body_envelope`: walk the body's outer structure (P1.2-b1)
//! - `stream_actors`: iterate every (object header + entity body) pair across all
//!   levels — zero-copy slices into the decompressed body (P1.2-b2)
//! - `parse_entity_body`: decode actor preamble + primitive properties from an
//!   entity body; composite property types stop iteration cleanly (P1.3-a)
//!
//! Roadmap: P1.3-b adds composite property types (Struct, Array, Map, Set, Enum,
//! Byte, Text). P1.3-c adds the `ClassDef` registry and the `Component` trait.

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
pub mod object_property;
pub use object_property::{read_object_property, ObjectProperty};
pub mod object_header;
pub use object_header::{
    read_object_header, read_objects_in_level, ObjectHeaderBody, ObjectKind, RawObjectHeader,
    Transform,
};
pub mod entity;
pub use entity::{read_entities_in_level, read_entity, RawEntity};
pub mod raw_actor;
pub use raw_actor::{stream_actors, RawActor};
pub mod property_guid;
pub use property_guid::{read_property_guid, PropertyGuid};
pub mod property;
pub use property::{
    read_properties, read_property, Property, PropertyBag, PropertyValue, UnsupportedHit,
};
pub mod entity_body;
pub use entity_body::{parse_entity_body, EntityBody};
