//! Stub — Task 2 implements `read_struct_property`.
#![allow(clippy::derive_partial_eq_without_eq)] // float-bearing variants land in Task 2

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::reader::Reader;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructValue {
    pub subtype: String,
    pub guid: Option<[u8; 16]>,
    pub has_index: u8,
    pub index: Option<i32>,
    pub kind: StructKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StructKind {
    OpaqueBlob(Vec<u8>),
}

pub fn read_struct_property(
    r: &mut Reader<'_>,
    _save_version: i32,
    _ue5_version: u32,
    _map_name: &str,
    _parent_type: Option<&str>,
    _length: i32,
) -> Result<StructValue> {
    // Stub: don't consume bytes; surface as Unsupported so the iterator stops cleanly
    // (same behavior as P1.3-a). Task 2 implements the real decoder.
    Err(Error::UnsupportedPropertyType {
        name: String::new(),
        type_name: "Struct".to_string(),
        at: r.position(),
    })
}
