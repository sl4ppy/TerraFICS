//! Array property dispatcher.
//!
//! After the standard property header: `element_type` string + 1 byte padding + i32 count,
//! then `count` elements whose layout depends on `element_type`. For Struct element type,
//! there is an additional preamble (see Task 4).
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:1450-1716.

#![allow(clippy::derive_partial_eq_without_eq)] // float-bearing variants

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::object_property::{read_object_property, ObjectProperty};
use crate::property_struct::StructKind;
use crate::property_text::{read_text_property, TextValue};
use crate::reader::Reader;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayValue {
    pub element_type: String,
    /// Set only when `element_type == "Struct"`; captures the inner property-header-like
    /// preamble that arrays of structs carry. Populated by Task 4.
    pub struct_outer: Option<ArrayStructOuter>,
    pub values: Vec<ArrayElement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayStructOuter {
    pub property_name: String,
    pub structure_size: i32,
    pub structure_subtype: String,
    pub struct_sub_guid: Option<[u8; 16]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArrayElement {
    Byte(u8),
    Bool(u8),
    Int(i32),
    Int64(i64),
    Float(f32),
    Double(f64),
    Enum(String),
    Str(String),
    Text(TextValue),
    ObjectRef(ObjectProperty),
    InterfaceRef(ObjectProperty),
    SoftObjectRef {
        path_name: String,
        sub_path_string: String,
    },
    Struct(StructKind),
}

pub fn read_array_property(
    r: &mut Reader<'_>,
    save_version: i32,
    ue5_version: u32,
    map_name: &str,
    parent_type: Option<&str>,
) -> Result<ArrayValue> {
    let element_type = if ue5_version < 1011 {
        let raw = r.read_string()?;
        raw.strip_suffix("Property").unwrap_or(&raw).to_string()
    } else {
        String::new()
    };

    let _padding = r.read_u8()?;
    let count = r.read_i32()?;
    let count_usize = usize::try_from(count.max(0)).unwrap_or(0);
    let mut values = Vec::with_capacity(count_usize);

    if element_type == "Struct" {
        // Task 4 will fill in. For now, surface as Unsupported so iterator stops cleanly.
        let _ = (save_version, map_name, parent_type);
        return Err(Error::UnsupportedPropertyType {
            name: String::new(),
            type_name: "Array<Struct>".to_string(),
            at: r.position(),
        });
    }

    for _ in 0..count_usize {
        let elem = match element_type.as_str() {
            "Byte" => ArrayElement::Byte(r.read_u8()?),
            "Bool" => ArrayElement::Bool(r.read_u8()?),
            "Int" => ArrayElement::Int(r.read_i32()?),
            "Int64" => ArrayElement::Int64(r.read_i64()?),
            "Float" => ArrayElement::Float(r.read_f32()?),
            "Double" => ArrayElement::Double(r.read_f64()?),
            "Enum" => ArrayElement::Enum(r.read_string()?),
            "Str" => ArrayElement::Str(r.read_string()?),
            "Name" => ArrayElement::Str(r.read_string()?),
            "Text" => ArrayElement::Text(read_text_property(r)?),
            "Object" => ArrayElement::ObjectRef(read_object_property(r, map_name)?),
            "Interface" => ArrayElement::InterfaceRef(read_object_property(r, map_name)?),
            "SoftObject" => {
                let path_name = r.read_string()?;
                let sub_path_string = r.read_string()?;
                let _trailer = r.read_i32()?;
                ArrayElement::SoftObjectRef {
                    path_name,
                    sub_path_string,
                }
            }
            other => {
                return Err(Error::UnsupportedPropertyType {
                    name: String::new(),
                    type_name: format!("Array<{other}>"),
                    at: r.position(),
                });
            }
        };
        values.push(elem);
    }

    Ok(ArrayValue {
        element_type,
        struct_outer: None,
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
    fn decodes_int_array() {
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "IntProperty");
        bytes.push(0);
        bytes.extend_from_slice(&3_i32.to_le_bytes());
        bytes.extend_from_slice(&10_i32.to_le_bytes());
        bytes.extend_from_slice(&20_i32.to_le_bytes());
        bytes.extend_from_slice(&30_i32.to_le_bytes());
        let mut r = Reader::new(&bytes);
        let v = read_array_property(&mut r, 46, 1000, "MapName", None).unwrap();
        assert_eq!(v.element_type, "Int");
        assert_eq!(v.values.len(), 3);
        assert_eq!(v.values[0], ArrayElement::Int(10));
        assert_eq!(v.values[2], ArrayElement::Int(30));
    }

    #[test]
    fn decodes_empty_array() {
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "FloatProperty");
        bytes.push(0);
        bytes.extend_from_slice(&0_i32.to_le_bytes());
        let mut r = Reader::new(&bytes);
        let v = read_array_property(&mut r, 46, 1000, "MapName", None).unwrap();
        assert_eq!(v.element_type, "Float");
        assert!(v.values.is_empty());
    }

    #[test]
    fn decodes_object_array() {
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "ObjectProperty");
        bytes.push(0);
        bytes.extend_from_slice(&2_i32.to_le_bytes());
        write_ascii(&mut bytes, "Persistent_Level");
        write_ascii(&mut bytes, "Persistent.Foo_1");
        write_ascii(&mut bytes, "Persistent_Level");
        write_ascii(&mut bytes, "Persistent.Foo_2");
        let mut r = Reader::new(&bytes);
        let v = read_array_property(&mut r, 46, 1000, "MapName", None).unwrap();
        assert_eq!(v.element_type, "Object");
        assert_eq!(v.values.len(), 2);
    }

    #[test]
    fn array_of_struct_returns_unsupported_until_task_4() {
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "StructProperty");
        bytes.push(0);
        bytes.extend_from_slice(&0_i32.to_le_bytes());
        let mut r = Reader::new(&bytes);
        let err = read_array_property(&mut r, 46, 1000, "MapName", None).unwrap_err();
        assert!(matches!(err, Error::UnsupportedPropertyType { .. }));
    }
}
