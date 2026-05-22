//! Stub — Task 8 implements `read_text_property`.
#![allow(clippy::derive_partial_eq_without_eq)] // Eq stays valid here but for parity with siblings

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

pub fn read_text_property(_r: &mut Reader<'_>) -> Result<TextValue> {
    Err(Error::UnsupportedPropertyType {
        name: String::new(),
        type_name: "Text".to_string(),
        at: 0,
    })
}
