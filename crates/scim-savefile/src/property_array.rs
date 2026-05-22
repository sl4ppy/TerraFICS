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
        let _ = parent_type;
        // Inner-struct preamble (ue5_version < 1011):
        let property_name = r.read_string()?;
        let _struct_prop_marker = r.read_string()?; // "StructProperty"
        let structure_size = r.read_i32()?;
        let _padding_i32 = r.read_i32()?; // 0
        let structure_subtype = r.read_string()?;
        let struct_sub_guid = r.read_guid()?;
        let _padding_u8 = r.read_u8()?;

        let outer = ArrayStructOuter {
            property_name,
            structure_size,
            structure_subtype: structure_subtype.clone(),
            struct_sub_guid,
        };

        let per_element_len = if count_usize > 0 {
            i32::try_from(usize::try_from(structure_size.max(0)).unwrap_or(0) / count_usize)
                .unwrap_or(0)
        } else {
            0
        };

        for _ in 0..count_usize {
            let elem = read_array_struct_element(
                r,
                save_version,
                ue5_version,
                map_name,
                &structure_subtype,
                per_element_len,
            )?;
            values.push(ArrayElement::Struct(elem));
        }

        return Ok(ArrayValue {
            element_type,
            struct_outer: Some(outer),
            values,
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

/// Read a single element of `Array<Struct>`. The struct subtype was set ONCE in the
/// array preamble (`structure_subtype`); per-element bytes are just the subtype-specific
/// body. `per_element_len` is used for opaque-blob fallback on unknown subtypes.
#[allow(clippy::too_many_lines)]
#[allow(clippy::many_single_char_names)]
fn read_array_struct_element(
    r: &mut Reader<'_>,
    save_version: i32,
    ue5_version: u32,
    map_name: &str,
    subtype: &str,
    per_element_len: i32,
) -> Result<StructKind> {
    let elem_start = r.position();
    let len_usize = usize::try_from(per_element_len.max(0)).unwrap_or(0);

    let kind = match subtype {
        "Vector" | "Rotator" => {
            let (x, y, z) = if save_version >= 41 {
                (r.read_f64()?, r.read_f64()?, r.read_f64()?)
            } else {
                (
                    f64::from(r.read_f32()?),
                    f64::from(r.read_f32()?),
                    f64::from(r.read_f32()?),
                )
            };
            if subtype == "Vector" {
                StructKind::Vector { x, y, z }
            } else {
                StructKind::Rotator { x, y, z }
            }
        }
        "Vector2D" => {
            let (x, y) = if save_version >= 41 {
                (r.read_f64()?, r.read_f64()?)
            } else {
                (f64::from(r.read_f32()?), f64::from(r.read_f32()?))
            };
            StructKind::Vector2D { x, y }
        }
        "Quat" | "Vector4" => {
            let (a, b, c, d) = if save_version >= 41 {
                (r.read_f64()?, r.read_f64()?, r.read_f64()?, r.read_f64()?)
            } else {
                (
                    f64::from(r.read_f32()?),
                    f64::from(r.read_f32()?),
                    f64::from(r.read_f32()?),
                    f64::from(r.read_f32()?),
                )
            };
            if subtype == "Quat" {
                StructKind::Quat { a, b, c, d }
            } else {
                StructKind::Vector4 { a, b, c, d }
            }
        }
        "LinearColor" => StructKind::LinearColor {
            r: r.read_f32()?,
            g: r.read_f32()?,
            b: r.read_f32()?,
            a: r.read_f32()?,
        },
        "Color" => StructKind::Color {
            b: r.read_u8()?,
            g: r.read_u8()?,
            r: r.read_u8()?,
            a: r.read_u8()?,
        },
        "Guid" => {
            let arr = r.read_array::<16>()?;
            StructKind::Guid(arr)
        }
        "IntPoint" => StructKind::IntPoint {
            x: r.read_i32()?,
            y: r.read_i32()?,
        },
        _ => {
            // Nested-property fallback per element. On any failure or unsupported nested
            // type, seek back and try an OpaqueBlob using per_element_len.
            match crate::property::read_properties(
                r,
                save_version,
                ue5_version,
                map_name,
                Some(subtype),
            ) {
                Ok(bag) if bag.first_unsupported.is_none() => StructKind::Nested(bag.properties),
                _ => {
                    r.seek(elem_start);
                    if len_usize > 0 {
                        match r.read_hex(len_usize) {
                            Ok(bytes) => StructKind::OpaqueBlob(bytes),
                            Err(_) => {
                                return Err(Error::UnsupportedPropertyType {
                                    name: String::new(),
                                    type_name: format!("Array<Struct<{subtype}>>"),
                                    at: elem_start,
                                });
                            }
                        }
                    } else {
                        return Err(Error::UnsupportedPropertyType {
                            name: String::new(),
                            type_name: format!("Array<Struct<{subtype}>>"),
                            at: elem_start,
                        });
                    }
                }
            }
        }
    };

    Ok(kind)
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
    fn decodes_array_of_vector_struct() {
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "StructProperty");
        bytes.push(0); // padding
        bytes.extend_from_slice(&2_i32.to_le_bytes()); // count = 2
        // Inner Struct preamble (ue5 < 1011 path):
        write_ascii(&mut bytes, "mVertexList"); // property_name
        write_ascii(&mut bytes, "StructProperty");
        bytes.extend_from_slice(&48_i32.to_le_bytes()); // structure_size = 2 * 24
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // padding
        write_ascii(&mut bytes, "Vector"); // structure_subtype
        bytes.extend_from_slice(&[0_u8; 16]); // struct_sub_guid (zero)
        bytes.push(0); // padding
        // Element 1
        bytes.extend_from_slice(&1.0_f64.to_le_bytes());
        bytes.extend_from_slice(&2.0_f64.to_le_bytes());
        bytes.extend_from_slice(&3.0_f64.to_le_bytes());
        // Element 2
        bytes.extend_from_slice(&4.0_f64.to_le_bytes());
        bytes.extend_from_slice(&5.0_f64.to_le_bytes());
        bytes.extend_from_slice(&6.0_f64.to_le_bytes());

        let mut r = Reader::new(&bytes);
        let v = read_array_property(&mut r, 46, 1000, "MapName", None).unwrap();
        assert_eq!(v.element_type, "Struct");
        let outer = v.struct_outer.expect("struct_outer");
        assert_eq!(outer.structure_subtype, "Vector");
        assert_eq!(v.values.len(), 2);
        if let ArrayElement::Struct(StructKind::Vector { x, y, z }) = &v.values[0] {
            assert!((x - 1.0).abs() < f64::EPSILON);
            assert!((y - 2.0).abs() < f64::EPSILON);
            assert!((z - 3.0).abs() < f64::EPSILON);
        } else {
            panic!("expected Struct(Vector) at index 0");
        }
    }
}
