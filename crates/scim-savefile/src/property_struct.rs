//! Struct property dispatcher.
//!
//! After the standard property header (name + type + length + index), a Struct's
//! value region begins with `subtype + struct_guid + has_index + [optional inner index]`
//! and then `length` bytes of subtype-specific body. We decode ~15 well-known fixed-
//! layout subtypes; everything else tries a recursive property-bag fallback and falls
//! back to an opaque blob if that fails.
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:2061-2378.

#![allow(clippy::derive_partial_eq_without_eq)] // float-bearing variants
#![allow(clippy::many_single_char_names)] // LinearColor / Quat / Vec4 use rgba, abcd by convention

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::property::{read_properties, Property};
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
    Vector {
        x: f64,
        y: f64,
        z: f64,
    },
    Rotator {
        x: f64,
        y: f64,
        z: f64,
    },
    Vector2D {
        x: f64,
        y: f64,
    },
    Vector4 {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
    },
    Quat {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
    },
    IntVector4 {
        a: i32,
        b: i32,
        c: i32,
        d: i32,
    },
    IntPoint {
        x: i32,
        y: i32,
    },
    LinearColor {
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    },
    Color {
        b: u8,
        g: u8,
        r: u8,
        a: u8,
    },
    Box {
        min: [f64; 3],
        max: [f64; 3],
        is_valid: u8,
    },
    Guid([u8; 16]),
    DateTime(i64),
    FluidBox(f32),
    TimerHandle(String),
    FICFrameRange {
        begin: i64,
        end: i64,
    },
    OpaqueBlob(Vec<u8>),
    Nested(Vec<Property>),
}

#[allow(clippy::too_many_lines)] // sequential dispatcher; splitting reduces clarity
pub fn read_struct_property(
    r: &mut Reader<'_>,
    save_version: i32,
    ue5_version: u32,
    map_name: &str,
    _parent_type: Option<&str>,
    length: i32,
) -> Result<StructValue> {
    let (subtype, guid) = if ue5_version < 1011 {
        let s = r.read_string()?;
        let g = r.read_guid()?;
        (s, g)
    } else {
        (String::new(), None)
    };

    let has_index = r.read_u8()?;
    let index = if has_index == 9 {
        Some(r.read_i32()?)
    } else {
        None
    };

    let value_start = r.position();
    let length_usize = usize::try_from(length.max(0)).unwrap_or(0);

    let kind = match subtype.as_str() {
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
        "Vector4" | "Quat" => {
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
        "IntVector4" => StructKind::IntVector4 {
            a: r.read_i32()?,
            b: r.read_i32()?,
            c: r.read_i32()?,
            d: r.read_i32()?,
        },
        "IntPoint" => StructKind::IntPoint {
            x: r.read_i32()?,
            y: r.read_i32()?,
        },
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
        "Box" => {
            let (min, max) = if save_version >= 41 {
                (
                    [r.read_f64()?, r.read_f64()?, r.read_f64()?],
                    [r.read_f64()?, r.read_f64()?, r.read_f64()?],
                )
            } else {
                (
                    [
                        f64::from(r.read_f32()?),
                        f64::from(r.read_f32()?),
                        f64::from(r.read_f32()?),
                    ],
                    [
                        f64::from(r.read_f32()?),
                        f64::from(r.read_f32()?),
                        f64::from(r.read_f32()?),
                    ],
                )
            };
            let is_valid = r.read_u8()?;
            StructKind::Box { min, max, is_valid }
        }
        "Guid" => {
            let arr = r.read_array::<16>()?;
            StructKind::Guid(arr)
        }
        "DateTime" => StructKind::DateTime(r.read_i64()?),
        "FluidBox" => StructKind::FluidBox(r.read_f32()?),
        "TimerHandle" => StructKind::TimerHandle(r.read_string()?),
        "FICFrameRange" => StructKind::FICFrameRange {
            begin: r.read_i64()?,
            end: r.read_i64()?,
        },
        "PlayerInfoHandle" | "UniqueNetIdRepl" | "ClientIdentityInfo" => {
            let bytes = r.read_hex(length_usize)?;
            StructKind::OpaqueBlob(bytes)
        }
        _ => {
            if let Ok(nested) = try_nested(
                r,
                save_version,
                ue5_version,
                map_name,
                &subtype,
                value_start,
                length_usize,
            ) {
                StructKind::Nested(nested)
            } else {
                r.seek(value_start);
                let bytes = r.read_hex(length_usize)?;
                StructKind::OpaqueBlob(bytes)
            }
        }
    };

    Ok(StructValue {
        subtype,
        guid,
        has_index,
        index,
        kind,
    })
}

fn try_nested(
    r: &mut Reader<'_>,
    save_version: i32,
    ue5_version: u32,
    map_name: &str,
    subtype: &str,
    value_start: usize,
    length_usize: usize,
) -> Result<Vec<Property>> {
    let bag = read_properties(r, save_version, ue5_version, map_name, Some(subtype))?;
    if bag.first_unsupported.is_some() {
        return Err(Error::UnsupportedPropertyType {
            name: String::new(),
            type_name: subtype.to_string(),
            at: value_start,
        });
    }
    if r.position() != value_start + length_usize {
        return Err(Error::ChunkLengthMismatch {
            at: value_start,
            expected: length_usize as u64,
            actual: (r.position() - value_start) as u64,
        });
    }
    Ok(bag.properties)
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

    /// Build the struct preamble: subtype + 16-byte zero `guid` + `has_index=0`.
    fn write_preamble(out: &mut Vec<u8>, subtype: &str) {
        write_ascii(out, subtype);
        out.extend_from_slice(&[0_u8; 16]); // zero guid
        out.push(0); // has_index = 0
    }

    #[test]
    fn decodes_vector_at_save_version_46() {
        let mut bytes = Vec::new();
        write_preamble(&mut bytes, "Vector");
        bytes.extend_from_slice(&1.0_f64.to_le_bytes());
        bytes.extend_from_slice(&2.0_f64.to_le_bytes());
        bytes.extend_from_slice(&3.0_f64.to_le_bytes());
        let mut r = Reader::new(&bytes);
        let v = read_struct_property(&mut r, 46, 1000, "MapName", None, 24).unwrap();
        assert_eq!(v.subtype, "Vector");
        match v.kind {
            StructKind::Vector { x, y, z } => {
                assert!((x - 1.0).abs() < f64::EPSILON);
                assert!((y - 2.0).abs() < f64::EPSILON);
                assert!((z - 3.0).abs() < f64::EPSILON);
            }
            other => panic!("expected Vector, got {other:?}"),
        }
    }

    #[test]
    fn decodes_linear_color() {
        let mut bytes = Vec::new();
        write_preamble(&mut bytes, "LinearColor");
        bytes.extend_from_slice(&0.25_f32.to_le_bytes());
        bytes.extend_from_slice(&0.5_f32.to_le_bytes());
        bytes.extend_from_slice(&0.75_f32.to_le_bytes());
        bytes.extend_from_slice(&1.0_f32.to_le_bytes());
        let mut r = Reader::new(&bytes);
        let v = read_struct_property(&mut r, 46, 1000, "MapName", None, 16).unwrap();
        match v.kind {
            StructKind::LinearColor { r, g, b, a } => {
                assert!((r - 0.25).abs() < f32::EPSILON);
                assert!((g - 0.5).abs() < f32::EPSILON);
                assert!((b - 0.75).abs() < f32::EPSILON);
                assert!((a - 1.0).abs() < f32::EPSILON);
            }
            other => panic!("expected LinearColor, got {other:?}"),
        }
    }

    #[test]
    fn decodes_color_bgra() {
        let mut bytes = Vec::new();
        write_preamble(&mut bytes, "Color");
        bytes.extend_from_slice(&[10_u8, 20, 30, 255]);
        let mut r = Reader::new(&bytes);
        let v = read_struct_property(&mut r, 46, 1000, "MapName", None, 4).unwrap();
        match v.kind {
            StructKind::Color { b, g, r, a } => {
                assert_eq!(b, 10);
                assert_eq!(g, 20);
                assert_eq!(r, 30);
                assert_eq!(a, 255);
            }
            other => panic!("expected Color, got {other:?}"),
        }
    }

    #[test]
    fn decodes_quat() {
        let mut bytes = Vec::new();
        write_preamble(&mut bytes, "Quat");
        for v in [0.0_f64, 0.0, 0.0, 1.0] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let mut r = Reader::new(&bytes);
        let v = read_struct_property(&mut r, 46, 1000, "MapName", None, 32).unwrap();
        match v.kind {
            StructKind::Quat { a, b, c, d } => {
                assert!(a.abs() < f64::EPSILON);
                assert!(b.abs() < f64::EPSILON);
                assert!(c.abs() < f64::EPSILON);
                assert!((d - 1.0).abs() < f64::EPSILON);
            }
            other => panic!("expected Quat, got {other:?}"),
        }
    }

    #[test]
    fn decodes_box() {
        let mut bytes = Vec::new();
        write_preamble(&mut bytes, "Box");
        for v in [-1.0_f64, -2.0, -3.0, 1.0, 2.0, 3.0] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes.push(1); // is_valid
        let mut r = Reader::new(&bytes);
        let v = read_struct_property(&mut r, 46, 1000, "MapName", None, 49).unwrap();
        match v.kind {
            StructKind::Box { min, max, is_valid } => {
                assert!((min[0] - (-1.0)).abs() < f64::EPSILON);
                assert!((max[2] - 3.0).abs() < f64::EPSILON);
                assert_eq!(is_valid, 1);
            }
            other => panic!("expected Box, got {other:?}"),
        }
    }

    #[test]
    fn unknown_subtype_with_no_inner_properties_returns_opaque_blob() {
        let mut bytes = Vec::new();
        write_preamble(&mut bytes, "WeirdMod");
        bytes.extend_from_slice(&[0xFF; 7]);
        let mut r = Reader::new(&bytes);
        let v = read_struct_property(&mut r, 46, 1000, "MapName", None, 7).unwrap();
        match v.kind {
            StructKind::OpaqueBlob(b) => assert_eq!(b.len(), 7),
            other => panic!("expected OpaqueBlob, got {other:?}"),
        }
    }
}
