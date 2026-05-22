//! Stub — Task 5 implements `read_map_property`.
#![allow(clippy::derive_partial_eq_without_eq)] // float-bearing variants land in Task 5

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::reader::Reader;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapValue {
    pub key_type: String,
    pub value_type: String,
    pub mode_type: i32,
    pub mode_unk1_hex: Option<Vec<u8>>,
    pub mode_unk2: Option<String>,
    pub mode_unk3: Option<String>,
    pub entries: Vec<MapEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapEntry {
    pub key: MapKey,
    pub value: MapValueEntry,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MapKey {
    Unimplemented,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MapValueEntry {
    Unimplemented,
}

pub fn read_map_property(
    _r: &mut Reader<'_>,
    _save_version: i32,
    _ue5_version: u32,
    _map_name: &str,
    _parent_type: Option<&str>,
) -> Result<MapValue> {
    Err(Error::UnsupportedPropertyType {
        name: String::new(),
        type_name: "Map".to_string(),
        at: 0,
    })
}
