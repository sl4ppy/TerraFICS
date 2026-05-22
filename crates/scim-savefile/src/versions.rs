//! Save-format version constants for `save_header_type`.
//! Cross-reference with the JS implementation at:
//!   SC-InteractiveMap/src/SaveParser/Read.js:30-69

/// Lowest header type we attempt to parse. Saves older than this are pre-Update-3 era and unsupported.
pub const MIN_SUPPORTED_HEADER_TYPE: i32 = 7;

/// Highest header type known at time of writing.
/// Update when adding support for a newer save format.
pub const MAX_KNOWN_HEADER_TYPE: i32 = 14;

pub const HAS_EDITOR_OBJECT_VERSION_FROM: i32 = 7;
pub const HAS_MOD_METADATA_FROM: i32 = 8;
pub const HAS_SAVE_IDENTIFIER_FROM: i32 = 10;
pub const HAS_PARTITIONED_WORLD_FROM: i32 = 13;
pub const HAS_SAVE_NAME_FROM: i32 = 14;
