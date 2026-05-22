//! Stub — Task 7 implements these.
#![allow(clippy::derive_partial_eq_without_eq)] // parity with sibling property_* modules

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::property::PropertyValue;
use crate::property_guid::PropertyGuid;
use crate::reader::Reader;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ByteValue {
    NoneValueU8(u8),
    EnumNamed { enum_name: String, value_name: String },
}

pub fn read_enum_property(
    _r: &mut Reader<'_>,
    _save_version: i32,
    _ue5_version: u32,
) -> Result<(PropertyGuid, PropertyValue)> {
    Err(Error::UnsupportedPropertyType {
        name: String::new(),
        type_name: "Enum".to_string(),
        at: 0,
    })
}

pub fn read_byte_property(
    _r: &mut Reader<'_>,
    _save_version: i32,
    _ue5_version: u32,
) -> Result<(PropertyGuid, PropertyValue)> {
    Err(Error::UnsupportedPropertyType {
        name: String::new(),
        type_name: "Byte".to_string(),
        at: 0,
    })
}
