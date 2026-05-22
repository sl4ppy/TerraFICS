//! Stub — Task 6 implements `read_set_property`.
#![allow(clippy::derive_partial_eq_without_eq)] // float-bearing variants land in Task 6

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::reader::Reader;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetValue {
    pub element_type: String,
    pub mode_type: i32,
    pub mode_unk1_hex: Option<Vec<u8>>,
    pub mode_unk2: Option<String>,
    pub mode_unk3: Option<String>,
    pub values: Vec<SetElement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SetElement {
    Unimplemented,
}

pub fn read_set_property(
    _r: &mut Reader<'_>,
    _save_version: i32,
    _ue5_version: u32,
    _map_name: &str,
    _parent_type: Option<&str>,
) -> Result<SetValue> {
    Err(Error::UnsupportedPropertyType {
        name: String::new(),
        type_name: "Set".to_string(),
        at: 0,
    })
}
