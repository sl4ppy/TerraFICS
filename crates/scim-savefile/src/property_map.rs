//! Map property dispatcher.
//!
//! After the property header: `key_type` + `value_type` strings + 1 byte padding +
//! `read_mode_type` (i32 mode + optional strings/hex) + i32 count + count pairs.
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:1717-1981.

#![allow(clippy::derive_partial_eq_without_eq)] // float-bearing variants

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::object_property::{read_object_property, ObjectProperty};
use crate::property::read_properties;
use crate::property_text::{read_text_property, TextValue};
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
    Int(i32),
    Int64(i64),
    Str(String),
    Name(String),
    Object(ObjectProperty),
    Enum(String),
    StructNested(Vec<crate::property::Property>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MapValueEntry {
    Byte(u8),
    ByteStr(String),
    Bool(u8),
    Int(i32),
    Int64(i64),
    Float(f32),
    Double(f64),
    IntVector { x: i32, y: i32, z: i32 },
    Str(String),
    Object(ObjectProperty),
    Text(TextValue),
    StructNested(Vec<crate::property::Property>),
}

/// Captured fields from `readModeType` — see JS Read.js:2529-2550.
#[derive(Debug, Clone, Default)]
pub struct ModeType {
    pub mode: i32,
    pub unk1_hex: Option<Vec<u8>>,
    pub unk2: Option<String>,
    pub unk3: Option<String>,
}

/// Shared `readModeType` parser used by Map and Set.
pub fn read_mode_type(r: &mut Reader<'_>) -> Result<ModeType> {
    let mode = r.read_i32()?;
    let mut out = ModeType {
        mode,
        ..ModeType::default()
    };
    if mode == 2 {
        out.unk2 = Some(r.read_string()?);
        out.unk3 = Some(r.read_string()?);
    } else if mode == 3 {
        out.unk1_hex = Some(r.read_hex(9)?);
        out.unk2 = Some(r.read_string()?);
        out.unk3 = Some(r.read_string()?);
    }
    Ok(out)
}

pub fn read_map_property(
    r: &mut Reader<'_>,
    save_version: i32,
    ue5_version: u32,
    map_name: &str,
    parent_type: Option<&str>,
) -> Result<MapValue> {
    let (key_type, value_type) = if ue5_version < 1011 {
        let k = r.read_string()?;
        let v = r.read_string()?;
        (
            k.strip_suffix("Property").unwrap_or(&k).to_string(),
            v.strip_suffix("Property").unwrap_or(&v).to_string(),
        )
    } else {
        (String::new(), String::new())
    };

    let _padding = r.read_u8()?;
    let mode = read_mode_type(r)?;
    let count = r.read_i32()?;
    let count_usize = usize::try_from(count.max(0)).unwrap_or(0);
    let mut entries = Vec::with_capacity(count_usize);

    for _ in 0..count_usize {
        let key = read_map_key(
            r,
            save_version,
            ue5_version,
            map_name,
            parent_type,
            &key_type,
        )?;
        let value = read_map_value(
            r,
            save_version,
            ue5_version,
            map_name,
            parent_type,
            &key_type,
            &value_type,
        )?;
        entries.push(MapEntry { key, value });
    }

    Ok(MapValue {
        key_type,
        value_type,
        mode_type: mode.mode,
        mode_unk1_hex: mode.unk1_hex,
        mode_unk2: mode.unk2,
        mode_unk3: mode.unk3,
        entries,
    })
}

fn read_map_key(
    r: &mut Reader<'_>,
    save_version: i32,
    ue5_version: u32,
    map_name: &str,
    _parent_type: Option<&str>,
    key_type: &str,
) -> Result<MapKey> {
    match key_type {
        "Int" => Ok(MapKey::Int(r.read_i32()?)),
        "Int64" => Ok(MapKey::Int64(r.read_i64()?)),
        "Name" => Ok(MapKey::Name(r.read_string()?)),
        "Str" => Ok(MapKey::Str(r.read_string()?)),
        "Object" => Ok(MapKey::Object(read_object_property(r, map_name)?)),
        "Enum" => Ok(MapKey::Enum(r.read_string()?)),
        "Struct" => {
            let bag = read_properties(r, save_version, ue5_version, map_name, Some("Struct"))?;
            if bag.first_unsupported.is_some() {
                return Err(Error::UnsupportedPropertyType {
                    name: String::new(),
                    type_name: "Map<Struct,...>".to_string(),
                    at: r.position(),
                });
            }
            Ok(MapKey::StructNested(bag.properties))
        }
        other => Err(Error::UnsupportedPropertyType {
            name: String::new(),
            type_name: format!("Map<{other},...>"),
            at: r.position(),
        }),
    }
}

fn read_map_value(
    r: &mut Reader<'_>,
    save_version: i32,
    ue5_version: u32,
    map_name: &str,
    _parent_type: Option<&str>,
    key_type: &str,
    value_type: &str,
) -> Result<MapValueEntry> {
    match value_type {
        "Byte" => {
            // Per JS Read.js:1844-1854, Map<Str,Byte> reads values as strings.
            if key_type == "Str" {
                Ok(MapValueEntry::ByteStr(r.read_string()?))
            } else {
                Ok(MapValueEntry::Byte(r.read_u8()?))
            }
        }
        "Bool" => Ok(MapValueEntry::Bool(r.read_u8()?)),
        "Int" => Ok(MapValueEntry::Int(r.read_i32()?)),
        "Int64" => Ok(MapValueEntry::Int64(r.read_i64()?)),
        "Float" => Ok(MapValueEntry::Float(r.read_f32()?)),
        "Double" => Ok(MapValueEntry::Double(r.read_f64()?)),
        "IntVector" => Ok(MapValueEntry::IntVector {
            x: r.read_i32()?,
            y: r.read_i32()?,
            z: r.read_i32()?,
        }),
        "Str" | "Name" => Ok(MapValueEntry::Str(r.read_string()?)),
        "Object" => Ok(MapValueEntry::Object(read_object_property(r, map_name)?)),
        "Text" => Ok(MapValueEntry::Text(read_text_property(r)?)),
        "Struct" => {
            let bag = read_properties(r, save_version, ue5_version, map_name, Some("Struct"))?;
            if bag.first_unsupported.is_some() {
                return Err(Error::UnsupportedPropertyType {
                    name: String::new(),
                    type_name: "Map<...,Struct>".to_string(),
                    at: r.position(),
                });
            }
            Ok(MapValueEntry::StructNested(bag.properties))
        }
        other => Err(Error::UnsupportedPropertyType {
            name: String::new(),
            type_name: format!("Map<...,{other}>"),
            at: r.position(),
        }),
    }
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
    fn decodes_int_to_str_map() {
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "IntProperty");
        write_ascii(&mut bytes, "StrProperty");
        bytes.push(0);
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // mode = 0
        bytes.extend_from_slice(&2_i32.to_le_bytes()); // count = 2
        bytes.extend_from_slice(&1_i32.to_le_bytes());
        write_ascii(&mut bytes, "one");
        bytes.extend_from_slice(&2_i32.to_le_bytes());
        write_ascii(&mut bytes, "two");
        let mut r = Reader::new(&bytes);
        let v = read_map_property(&mut r, 46, 1000, "MapName", None).unwrap();
        assert_eq!(v.key_type, "Int");
        assert_eq!(v.value_type, "Str");
        assert_eq!(v.mode_type, 0);
        assert_eq!(v.entries.len(), 2);
        assert_eq!(v.entries[0].key, MapKey::Int(1));
        assert_eq!(v.entries[0].value, MapValueEntry::Str("one".to_string()));
    }

    #[test]
    fn decodes_str_to_int_map() {
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "StrProperty");
        write_ascii(&mut bytes, "IntProperty");
        bytes.push(0);
        bytes.extend_from_slice(&0_i32.to_le_bytes());
        bytes.extend_from_slice(&1_i32.to_le_bytes());
        write_ascii(&mut bytes, "foo");
        bytes.extend_from_slice(&42_i32.to_le_bytes());
        let mut r = Reader::new(&bytes);
        let v = read_map_property(&mut r, 46, 1000, "MapName", None).unwrap();
        assert_eq!(v.entries.len(), 1);
        assert_eq!(v.entries[0].key, MapKey::Str("foo".to_string()));
        assert_eq!(v.entries[0].value, MapValueEntry::Int(42));
    }
}
