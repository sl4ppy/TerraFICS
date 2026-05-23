//! Property records inside an entity's property bag.
//!
//! Each record is `name (string) + type (string) + length (i32) + [index (i32) if
//! ue5_version < 1011] + type-specific preamble + value`. Iteration ends when a
//! property's name string reads as the literal `"None"`.
//!
//! P1.3-a decodes only primitive value types; composite types (Struct, Array, Map,
//! Set, Enum, Byte, Text) cause `read_property` to return
//! `Error::UnsupportedPropertyType` so the caller can stop the iterator cleanly.
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:1139-1448.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::object_property::{read_object_property, ObjectProperty};
use crate::property_guid::{read_property_guid, PropertyGuid};
use crate::reader::Reader;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    /// UE type name with the trailing `"Property"` stripped — e.g. `"Int"`, `"Float"`,
    /// `"StructProperty"` becomes `"Struct"`.
    pub type_name: String,
    /// Per-property index (almost always 0). Only present when `ue5_version < 1011`.
    pub index: Option<i32>,
    pub guid: Option<PropertyGuid>,
    pub value: PropertyValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyValue {
    // Primitives (P1.3-a)
    Bool(bool),
    Int8(i8),
    Int(i32),
    UInt32(u32),
    Int64(i64),
    UInt64(u64),
    Float(f32),
    Double(f64),
    Str(String),
    Name(String),
    ObjectRef(ObjectProperty),
    InterfaceRef(ObjectProperty),
    SoftObjectRef {
        path_name: String,
        sub_path_string: String,
    },

    // Composites (P1.3-b — wired in subsequent tasks)
    Struct(crate::property_struct::StructValue),
    Array(crate::property_array::ArrayValue),
    Map(crate::property_map::MapValue),
    Set(crate::property_set::SetValue),
    Enum {
        enum_name: String,
        value: String,
    },
    Byte(crate::property_enum_byte::ByteValue),
    Text(crate::property_text::TextValue),
}

impl PropertyValue {
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_int(&self) -> Option<i32> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_int64(&self) -> Option<i64> {
        match self {
            Self::Int64(v) => Some(*v),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_float(&self) -> Option<f32> {
        match self {
            Self::Float(v) => Some(*v),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_double(&self) -> Option<f64> {
        match self {
            Self::Double(v) => Some(*v),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) | Self::Name(s) => Some(s.as_str()),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_object_ref(&self) -> Option<&ObjectProperty> {
        match self {
            Self::ObjectRef(o) | Self::InterfaceRef(o) => Some(o),
            _ => None,
        }
    }

    /// Read a Byte property's `EnumNamed` variant value name.
    /// Returns None for the `NoneValueU8` variant or non-Byte types.
    #[must_use]
    pub fn as_byte_enum(&self) -> Option<&str> {
        match self {
            Self::Byte(crate::property_enum_byte::ByteValue::EnumNamed { value_name, .. }) => {
                Some(value_name.as_str())
            }
            _ => None,
        }
    }
}

/// Read the next property record from `r`. Returns `Ok(None)` if the next name
/// string is the literal `"None"` sentinel (end of property bag).
///
/// Caller must pass:
/// - `save_version`: the level's `save_version` (must be `< 53` for P1.3-a).
/// - `ue5_version`: the level's UE5 version (controls the `index` field path).
/// - `map_name`: the save's `map_name`, needed by `read_object_property` for the
///   collapsed-form check.
#[allow(clippy::too_many_lines)] // sequential type-name dispatcher; splitting would obscure flow
pub fn read_property(
    r: &mut Reader<'_>,
    save_version: i32,
    ue5_version: u32,
    map_name: &str,
    parent_type: Option<&str>,
) -> Result<Option<Property>> {
    if save_version >= 53 {
        return Err(Error::UnsupportedPropertyFormat { save_version });
    }

    let property_start = r.position();
    let name = r.read_string()?;
    if name == "None" {
        return Ok(None);
    }

    let type_name_raw = r.read_string()?;
    let type_name = type_name_raw
        .strip_suffix("Property")
        .unwrap_or(type_name_raw.as_str())
        .to_string();

    let length = r.read_i32()?;

    let index = if ue5_version < 1011 {
        Some(r.read_i32()?)
    } else {
        None
    };

    // Dispatch by type_name. P1.3-a primitives only.
    // Each branch handles its own propertyGUID placement (Bool's is AFTER value,
    // all others' are BEFORE) — per JS Read.js:1264-1424.
    let (guid, value) = match type_name.as_str() {
        "Bool" => {
            // Bool reads its value byte BEFORE the propertyGUID. JS at line 1267-1272
            // also rewrites the value `16` to `1`; we treat any non-zero byte as true,
            // which preserves the boolean semantics and absorbs the `16` quirk.
            let raw = r.read_u8()?;
            let guid = read_property_guid(r)?;
            (Some(guid), PropertyValue::Bool(raw != 0))
        }
        "Int8" => {
            let guid = read_property_guid(r)?;
            (Some(guid), PropertyValue::Int8(r.read_i8()?))
        }
        "Int" => {
            let guid = read_property_guid(r)?;
            (Some(guid), PropertyValue::Int(r.read_i32()?))
        }
        "UInt32" => {
            let guid = read_property_guid(r)?;
            (Some(guid), PropertyValue::UInt32(r.read_u32()?))
        }
        "Int64" => {
            let guid = read_property_guid(r)?;
            (Some(guid), PropertyValue::Int64(r.read_i64()?))
        }
        "UInt64" => {
            let guid = read_property_guid(r)?;
            (Some(guid), PropertyValue::UInt64(r.read_u64()?))
        }
        "Float" => {
            let guid = read_property_guid(r)?;
            (Some(guid), PropertyValue::Float(r.read_f32()?))
        }
        "Double" => {
            let guid = read_property_guid(r)?;
            (Some(guid), PropertyValue::Double(r.read_f64()?))
        }
        "Str" => {
            let guid = read_property_guid(r)?;
            (Some(guid), PropertyValue::Str(r.read_string()?))
        }
        "Name" => {
            let guid = read_property_guid(r)?;
            (Some(guid), PropertyValue::Name(r.read_string()?))
        }
        "Object" => {
            let guid = read_property_guid(r)?;
            let obj = read_object_property(r, map_name)?;
            (Some(guid), PropertyValue::ObjectRef(obj))
        }
        "Interface" => {
            let guid = read_property_guid(r)?;
            let obj = read_object_property(r, map_name)?;
            (Some(guid), PropertyValue::InterfaceRef(obj))
        }
        "SoftObject" => {
            let guid = read_property_guid(r)?;
            let path_name = r.read_string()?;
            let sub_path_string = r.read_string()?;
            let _trailer = r.read_i32()?; // always 0 in observed saves
            (
                Some(guid),
                PropertyValue::SoftObjectRef {
                    path_name,
                    sub_path_string,
                },
            )
        }
        "Struct" => {
            let value = crate::property_struct::read_struct_property(
                r,
                save_version,
                ue5_version,
                map_name,
                parent_type,
                length,
            )?;
            (None, PropertyValue::Struct(value))
        }
        "Array" => {
            let value = crate::property_array::read_array_property(
                r,
                save_version,
                ue5_version,
                map_name,
                parent_type,
            )?;
            (None, PropertyValue::Array(value))
        }
        "Map" => {
            let value = crate::property_map::read_map_property(
                r,
                save_version,
                ue5_version,
                map_name,
                parent_type,
            )?;
            (None, PropertyValue::Map(value))
        }
        "Set" => {
            let value = crate::property_set::read_set_property(
                r,
                save_version,
                ue5_version,
                map_name,
                parent_type,
            )?;
            (None, PropertyValue::Set(value))
        }
        "Enum" => {
            let (guid, value) =
                crate::property_enum_byte::read_enum_property(r, save_version, ue5_version)?;
            (Some(guid), value)
        }
        "Byte" => {
            let (guid, value) =
                crate::property_enum_byte::read_byte_property(r, save_version, ue5_version)?;
            (Some(guid), value)
        }
        "Text" => {
            let guid = read_property_guid(r)?;
            let text = crate::property_text::read_text_property(r)?;
            (Some(guid), PropertyValue::Text(text))
        }
        _ => {
            return Err(Error::UnsupportedPropertyType {
                name,
                type_name,
                at: property_start,
            });
        }
    };

    Ok(Some(Property {
        name,
        type_name,
        index,
        guid,
        value,
    }))
}

/// Result of walking a property bag.
#[derive(Debug, Default)]
pub struct PropertyBag {
    pub properties: Vec<Property>,
    /// Set when the iterator stopped because of an unsupported property type (rather
    /// than the `"None"` sentinel). Holds the type name so callers can tally what's
    /// missing in P1.3-a.
    pub first_unsupported: Option<UnsupportedHit>,
}

#[derive(Debug, Clone)]
pub struct UnsupportedHit {
    pub property_name: String,
    pub type_name: String,
    pub at: usize,
}

/// Loop `read_property` until the `"None"` sentinel OR an `UnsupportedPropertyType`
/// error.
pub fn read_properties(
    r: &mut Reader<'_>,
    save_version: i32,
    ue5_version: u32,
    map_name: &str,
    parent_type: Option<&str>,
) -> Result<PropertyBag> {
    let mut bag = PropertyBag::default();
    loop {
        match read_property(r, save_version, ue5_version, map_name, parent_type) {
            Ok(Some(p)) => bag.properties.push(p),
            Ok(None) => return Ok(bag),
            Err(Error::UnsupportedPropertyType {
                name,
                type_name,
                at,
            }) => {
                bag.first_unsupported = Some(UnsupportedHit {
                    property_name: name,
                    type_name,
                    at,
                });
                return Ok(bag);
            }
            Err(other) => return Err(other),
        }
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
    fn none_sentinel_returns_ok_none() {
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "None");
        let mut r = Reader::new(&bytes);
        let result = read_property(&mut r, 46, 1000, "MapName", None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn rejects_save_version_53() {
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "AnyName");
        let mut r = Reader::new(&bytes);
        let err = read_property(&mut r, 53, 1000, "MapName", None).unwrap_err();
        assert!(matches!(
            err,
            Error::UnsupportedPropertyFormat { save_version: 53 }
        ));
    }

    /// Build the (name, type, length, index) header for a property at `ue5_version` < 1011.
    fn write_header(out: &mut Vec<u8>, name: &str, type_property: &str, length: i32, index: i32) {
        write_ascii(out, name);
        write_ascii(out, type_property);
        out.extend_from_slice(&length.to_le_bytes());
        out.extend_from_slice(&index.to_le_bytes());
    }

    /// propertyGUID indicator byte = 0 (no GUID).
    fn write_no_guid(out: &mut Vec<u8>) {
        out.push(0);
    }

    #[test]
    fn decodes_int_property() {
        let mut bytes = Vec::new();
        write_header(&mut bytes, "mCount", "IntProperty", 4, 0);
        write_no_guid(&mut bytes);
        bytes.extend_from_slice(&42_i32.to_le_bytes());
        let mut r = Reader::new(&bytes);
        let p = read_property(&mut r, 46, 1000, "MapName", None)
            .unwrap()
            .unwrap();
        assert_eq!(p.name, "mCount");
        assert_eq!(p.type_name, "Int");
        assert_eq!(p.index, Some(0));
        assert_eq!(p.value, PropertyValue::Int(42));
    }

    #[test]
    fn decodes_float_property() {
        let mut bytes = Vec::new();
        write_header(&mut bytes, "mFuelLevel", "FloatProperty", 4, 0);
        write_no_guid(&mut bytes);
        bytes.extend_from_slice(&3.5_f32.to_le_bytes());
        let mut r = Reader::new(&bytes);
        let p = read_property(&mut r, 46, 1000, "MapName", None)
            .unwrap()
            .unwrap();
        assert_eq!(p.type_name, "Float");
        match p.value {
            PropertyValue::Float(v) => assert!((v - 3.5).abs() < f32::EPSILON),
            other => panic!("expected Float, got {other:?}"),
        }
    }

    #[test]
    fn decodes_double_property() {
        let mut bytes = Vec::new();
        write_header(&mut bytes, "mPosX", "DoubleProperty", 8, 0);
        write_no_guid(&mut bytes);
        bytes.extend_from_slice(&100.25_f64.to_le_bytes());
        let mut r = Reader::new(&bytes);
        let p = read_property(&mut r, 46, 1000, "MapName", None)
            .unwrap()
            .unwrap();
        assert_eq!(p.type_name, "Double");
        match p.value {
            PropertyValue::Double(v) => assert!((v - 100.25).abs() < f64::EPSILON),
            other => panic!("expected Double, got {other:?}"),
        }
    }

    #[test]
    fn decodes_int64_property() {
        let mut bytes = Vec::new();
        write_header(&mut bytes, "mTimestamp", "Int64Property", 8, 0);
        write_no_guid(&mut bytes);
        bytes.extend_from_slice(&(-7_i64).to_le_bytes());
        let mut r = Reader::new(&bytes);
        let p = read_property(&mut r, 46, 1000, "MapName", None)
            .unwrap()
            .unwrap();
        assert_eq!(p.type_name, "Int64");
        assert_eq!(p.value, PropertyValue::Int64(-7));
    }

    #[test]
    fn decodes_str_property() {
        let mut bytes = Vec::new();
        write_header(&mut bytes, "mLabel", "StrProperty", 0, 0);
        write_no_guid(&mut bytes);
        write_ascii(&mut bytes, "hello");
        let mut r = Reader::new(&bytes);
        let p = read_property(&mut r, 46, 1000, "MapName", None)
            .unwrap()
            .unwrap();
        assert_eq!(p.type_name, "Str");
        assert_eq!(p.value, PropertyValue::Str("hello".to_string()));
    }

    #[test]
    fn decodes_bool_property_true_when_byte_is_one() {
        let mut bytes = Vec::new();
        write_header(&mut bytes, "mEnabled", "BoolProperty", 0, 0);
        bytes.push(1); // value
        write_no_guid(&mut bytes);
        let mut r = Reader::new(&bytes);
        let p = read_property(&mut r, 46, 1000, "MapName", None)
            .unwrap()
            .unwrap();
        assert_eq!(p.value, PropertyValue::Bool(true));
    }

    #[test]
    fn decodes_bool_property_false_when_byte_is_zero() {
        let mut bytes = Vec::new();
        write_header(&mut bytes, "mEnabled", "BoolProperty", 0, 0);
        bytes.push(0); // value
        write_no_guid(&mut bytes);
        let mut r = Reader::new(&bytes);
        let p = read_property(&mut r, 46, 1000, "MapName", None)
            .unwrap()
            .unwrap();
        assert_eq!(p.value, PropertyValue::Bool(false));
    }

    #[test]
    fn decodes_object_property() {
        let mut bytes = Vec::new();
        write_header(&mut bytes, "mOwner", "ObjectProperty", 0, 0);
        write_no_guid(&mut bytes);
        // ObjectProperty: 2 strings (expanded form: level != map_name)
        write_ascii(&mut bytes, "Persistent_Level");
        write_ascii(&mut bytes, "Persistent.Owner_42");
        let mut r = Reader::new(&bytes);
        let p = read_property(&mut r, 46, 1000, "MapName", None)
            .unwrap()
            .unwrap();
        assert_eq!(p.type_name, "Object");
        match p.value {
            PropertyValue::ObjectRef(obj) => {
                assert_eq!(obj.level_name.as_deref(), Some("Persistent_Level"));
                assert_eq!(obj.path_name, "Persistent.Owner_42");
            }
            other => panic!("expected ObjectRef, got {other:?}"),
        }
    }

    #[test]
    fn decodes_soft_object_property() {
        let mut bytes = Vec::new();
        write_header(&mut bytes, "mSpawn", "SoftObjectProperty", 0, 0);
        write_no_guid(&mut bytes);
        write_ascii(&mut bytes, "/Game/Path");
        write_ascii(&mut bytes, "SubPath");
        bytes.extend_from_slice(&0_i32.to_le_bytes()); // trailer
        let mut r = Reader::new(&bytes);
        let p = read_property(&mut r, 46, 1000, "MapName", None)
            .unwrap()
            .unwrap();
        assert_eq!(p.type_name, "SoftObject");
        match p.value {
            PropertyValue::SoftObjectRef {
                path_name,
                sub_path_string,
            } => {
                assert_eq!(path_name, "/Game/Path");
                assert_eq!(sub_path_string, "SubPath");
            }
            other => panic!("expected SoftObjectRef, got {other:?}"),
        }
    }

    #[test]
    fn iterates_until_none_sentinel() {
        let mut bytes = Vec::new();
        // Property 1: Int
        write_header(&mut bytes, "mA", "IntProperty", 4, 0);
        write_no_guid(&mut bytes);
        bytes.extend_from_slice(&1_i32.to_le_bytes());
        // Property 2: Float
        write_header(&mut bytes, "mB", "FloatProperty", 4, 0);
        write_no_guid(&mut bytes);
        bytes.extend_from_slice(&2.5_f32.to_le_bytes());
        // None sentinel
        write_ascii(&mut bytes, "None");

        let mut r = Reader::new(&bytes);
        let bag = read_properties(&mut r, 46, 1000, "MapName", None).unwrap();
        assert_eq!(bag.properties.len(), 2);
        assert_eq!(bag.properties[0].name, "mA");
        assert_eq!(bag.properties[1].name, "mB");
        assert!(bag.first_unsupported.is_none());
    }

    #[test]
    fn iterator_stops_on_unsupported_type() {
        let mut bytes = Vec::new();
        // Property 1: Int (succeeds)
        write_header(&mut bytes, "mA", "IntProperty", 4, 0);
        write_no_guid(&mut bytes);
        bytes.extend_from_slice(&1_i32.to_le_bytes());
        // Property 2: an unknown type that remains unsupported after P1.3-b
        write_header(&mut bytes, "mList", "WeirdModProperty", 0, 0);

        let mut r = Reader::new(&bytes);
        let bag = read_properties(&mut r, 46, 1000, "MapName", None).unwrap();
        assert_eq!(bag.properties.len(), 1);
        let hit = bag.first_unsupported.expect("expected unsupported hit");
        assert_eq!(hit.property_name, "mList");
        assert_eq!(hit.type_name, "WeirdMod");
    }

    #[test]
    fn as_int_extracts_int_value() {
        assert_eq!(PropertyValue::Int(42).as_int(), Some(42));
        assert_eq!(PropertyValue::Float(1.0).as_int(), None);
    }

    #[test]
    fn as_str_handles_str_and_name() {
        assert_eq!(
            PropertyValue::Str("hello".to_string()).as_str(),
            Some("hello")
        );
        assert_eq!(
            PropertyValue::Name("named".to_string()).as_str(),
            Some("named")
        );
        assert_eq!(PropertyValue::Int(7).as_str(), None);
    }

    #[test]
    fn as_object_ref_handles_object_and_interface() {
        let obj = ObjectProperty {
            level_name: None,
            path_name: "Foo".to_string(),
        };
        let val = PropertyValue::ObjectRef(obj.clone());
        assert_eq!(
            val.as_object_ref().map(|o| o.path_name.as_str()),
            Some("Foo")
        );
        let val2 = PropertyValue::InterfaceRef(obj);
        assert_eq!(
            val2.as_object_ref().map(|o| o.path_name.as_str()),
            Some("Foo")
        );
        assert_eq!(PropertyValue::Int(1).as_object_ref(), None);
    }

    #[test]
    fn unsupported_type_returns_error() {
        let mut bytes = Vec::new();
        write_header(&mut bytes, "mWeird", "WeirdModProperty", 0, 0);
        let mut r = Reader::new(&bytes);
        let err = read_property(&mut r, 46, 1000, "MapName", None).unwrap_err();
        match err {
            Error::UnsupportedPropertyType {
                name, type_name, ..
            } => {
                assert_eq!(name, "mWeird");
                assert_eq!(type_name, "WeirdMod");
            }
            other => panic!("expected UnsupportedPropertyType, got {other:?}"),
        }
    }
}
