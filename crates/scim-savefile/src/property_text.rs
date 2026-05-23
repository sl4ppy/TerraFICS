//! Text property reader.
//!
//! P1.3-b implements only `HISTORYTYPE_BASE = 0` (the most common case). Other
//! history types surface as `Error::UnsupportedPropertyType` so the iterator stops
//! cleanly with the type name available for diagnostics.
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:2380-2473.

#![allow(clippy::derive_partial_eq_without_eq)] // parity with sibling property_* modules

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::reader::Reader;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextValue {
    pub flags: i32,
    pub history_type: u8,
    pub kind: TextKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TextKind {
    Base {
        namespace: String,
        key: String,
        value: String,
    },
    Unimplemented {
        history_type: u8,
    },
}

pub fn read_text_property(r: &mut Reader<'_>) -> Result<TextValue> {
    let flags = r.read_i32()?;
    let history_type = r.read_u8()?;
    let kind = match history_type {
        0 => {
            let namespace = r.read_string()?;
            let key = r.read_string()?;
            let value = r.read_string()?;
            TextKind::Base {
                namespace,
                key,
                value,
            }
        }
        other => {
            return Err(Error::UnsupportedPropertyType {
                name: String::new(),
                type_name: format!("Text<historyType={other}>"),
                at: r.position(),
            });
        }
    };
    Ok(TextValue {
        flags,
        history_type,
        kind,
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
    fn decodes_history_type_base() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0_i32.to_le_bytes());
        bytes.push(0);
        write_ascii(&mut bytes, "namespace_foo");
        write_ascii(&mut bytes, "key_bar");
        write_ascii(&mut bytes, "value_baz");
        let mut r = Reader::new(&bytes);
        let v = read_text_property(&mut r).unwrap();
        assert_eq!(v.flags, 0);
        assert_eq!(v.history_type, 0);
        match v.kind {
            TextKind::Base {
                namespace,
                key,
                value,
            } => {
                assert_eq!(namespace, "namespace_foo");
                assert_eq!(key, "key_bar");
                assert_eq!(value, "value_baz");
            }
            other @ TextKind::Unimplemented { .. } => panic!("expected Base, got {other:?}"),
        }
    }

    #[test]
    fn rejects_non_base_history_types() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0_i32.to_le_bytes());
        bytes.push(1);
        let mut r = Reader::new(&bytes);
        let err = read_text_property(&mut r).unwrap_err();
        assert!(matches!(err, Error::UnsupportedPropertyType { .. }));
    }
}
