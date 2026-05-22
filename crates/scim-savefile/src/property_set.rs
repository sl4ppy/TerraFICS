//! Set property dispatcher.
//!
//! After the property header: `element_type` string + 1 byte padding + `read_mode_type`
//! + i32 count + count elements.
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:1983-2059.

#![allow(clippy::derive_partial_eq_without_eq)] // float-bearing variants

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::object_property::{read_object_property, ObjectProperty};
use crate::property_map::read_mode_type;
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
    Object(ObjectProperty),
    Name(String),
    Str(String),
    Int(i32),
    UInt32(u32),
    StructFoliageRemoval { x: f32, y: f32, z: f32 },
    StructScannableGuid([u8; 16]),
    StructNested(Vec<crate::property::Property>),
}

pub fn read_set_property(
    r: &mut Reader<'_>,
    save_version: i32,
    ue5_version: u32,
    map_name: &str,
    parent_type: Option<&str>,
) -> Result<SetValue> {
    let element_type = if ue5_version < 1011 {
        let s = r.read_string()?;
        s.strip_suffix("Property").unwrap_or(&s).to_string()
    } else {
        String::new()
    };

    let _padding = r.read_u8()?;
    let mode = read_mode_type(r)?;
    let count = r.read_i32()?;
    let count_usize = usize::try_from(count.max(0)).unwrap_or(0);
    let mut values = Vec::with_capacity(count_usize);

    for _ in 0..count_usize {
        let elem = match element_type.as_str() {
            "Object" => SetElement::Object(read_object_property(r, map_name)?),
            "Name" => SetElement::Name(r.read_string()?),
            "Str" => SetElement::Str(r.read_string()?),
            "Int" => SetElement::Int(r.read_i32()?),
            "UInt32" => SetElement::UInt32(r.read_u32()?),
            "Struct" => {
                if parent_type == Some("/Script/FactoryGame.FGFoliageRemoval") {
                    SetElement::StructFoliageRemoval {
                        x: r.read_f32()?,
                        y: r.read_f32()?,
                        z: r.read_f32()?,
                    }
                } else if parent_type == Some("/Script/FactoryGame.FGScannableSubsystem") {
                    let arr = r.read_array::<16>()?;
                    SetElement::StructScannableGuid(arr)
                } else {
                    let bag = crate::property::read_properties(
                        r,
                        save_version,
                        ue5_version,
                        map_name,
                        Some("Struct"),
                    )?;
                    if bag.first_unsupported.is_some() {
                        return Err(Error::UnsupportedPropertyType {
                            name: String::new(),
                            type_name: "Set<Struct>".to_string(),
                            at: r.position(),
                        });
                    }
                    SetElement::StructNested(bag.properties)
                }
            }
            other => {
                return Err(Error::UnsupportedPropertyType {
                    name: String::new(),
                    type_name: format!("Set<{other}>"),
                    at: r.position(),
                });
            }
        };
        values.push(elem);
    }

    Ok(SetValue {
        element_type,
        mode_type: mode.mode,
        mode_unk1_hex: mode.unk1_hex,
        mode_unk2: mode.unk2,
        mode_unk3: mode.unk3,
        values,
    })
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
    fn decodes_int_set() {
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "IntProperty");
        bytes.push(0);
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // mode = 0
        bytes.extend_from_slice(&3_i32.to_le_bytes()); // count = 3
        bytes.extend_from_slice(&1_i32.to_le_bytes());
        bytes.extend_from_slice(&2_i32.to_le_bytes());
        bytes.extend_from_slice(&3_i32.to_le_bytes());
        let mut r = Reader::new(&bytes);
        let v = read_set_property(&mut r, 46, 1000, "MapName", None).unwrap();
        assert_eq!(v.element_type, "Int");
        assert_eq!(v.values.len(), 3);
        assert_eq!(v.values[0], SetElement::Int(1));
    }
}
