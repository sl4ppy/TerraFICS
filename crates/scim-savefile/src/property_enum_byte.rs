//! Enum + Byte property dispatchers (old format, `ue5_version < 1011`).
//!
//! Enum: 1 string (`enum_name`) + `propertyGuid` + 1 string (value).
//! Byte: 1 string (`enum_name`) + `propertyGuid` + (if `enum_name == "None"`: 1 byte value;
//!       else: 1 string `value_name`).
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:1332-1346 (Enum),
//! Read.js:1366-1387 (Byte).

#![allow(clippy::derive_partial_eq_without_eq)] // parity with sibling property_* modules

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::property::PropertyValue;
use crate::property_guid::{read_property_guid, PropertyGuid};
use crate::reader::Reader;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ByteValue {
    NoneValueU8(u8),
    EnumNamed {
        enum_name: String,
        value_name: String,
    },
}

pub fn read_enum_property(
    r: &mut Reader<'_>,
    _save_version: i32,
    _ue5_version: u32,
) -> Result<(PropertyGuid, PropertyValue)> {
    let enum_name = r.read_string()?;
    let guid = read_property_guid(r)?;
    let value = r.read_string()?;
    Ok((guid, PropertyValue::Enum { enum_name, value }))
}

pub fn read_byte_property(
    r: &mut Reader<'_>,
    _save_version: i32,
    _ue5_version: u32,
) -> Result<(PropertyGuid, PropertyValue)> {
    let enum_name = r.read_string()?;
    let guid = read_property_guid(r)?;
    let value = if enum_name == "None" {
        ByteValue::NoneValueU8(r.read_u8()?)
    } else {
        let value_name = r.read_string()?;
        ByteValue::EnumNamed {
            enum_name,
            value_name,
        }
    };
    Ok((guid, PropertyValue::Byte(value)))
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

    #[test]
    fn decodes_enum_property() {
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "EOutputType::Type");
        bytes.push(0);
        write_ascii(&mut bytes, "EOutputType::OT_Production");
        let mut r = Reader::new(&bytes);
        let (_, v) = read_enum_property(&mut r, 46, 1000).unwrap();
        if let PropertyValue::Enum { enum_name, value } = v {
            assert_eq!(enum_name, "EOutputType::Type");
            assert_eq!(value, "EOutputType::OT_Production");
        } else {
            panic!("expected Enum");
        }
    }

    #[test]
    fn decodes_byte_property_none_path() {
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "None");
        bytes.push(0);
        bytes.push(42);
        let mut r = Reader::new(&bytes);
        let (_, v) = read_byte_property(&mut r, 46, 1000).unwrap();
        if let PropertyValue::Byte(ByteValue::NoneValueU8(b)) = v {
            assert_eq!(b, 42);
        } else {
            panic!("expected Byte::NoneValueU8");
        }
    }

    #[test]
    fn decodes_byte_property_enum_path() {
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "ELoadingMode");
        bytes.push(0);
        write_ascii(&mut bytes, "ELoadingMode::Loaded");
        let mut r = Reader::new(&bytes);
        let (_, v) = read_byte_property(&mut r, 46, 1000).unwrap();
        if let PropertyValue::Byte(ByteValue::EnumNamed {
            enum_name,
            value_name,
        }) = v
        {
            assert_eq!(enum_name, "ELoadingMode");
            assert_eq!(value_name, "ELoadingMode::Loaded");
        } else {
            panic!("expected Byte::EnumNamed");
        }
    }
}
