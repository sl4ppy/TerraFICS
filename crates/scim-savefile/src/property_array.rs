//! Stub — Task 3/4 implement `read_array_property`.
#![allow(clippy::derive_partial_eq_without_eq)] // float-bearing variants land in Task 3

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::reader::Reader;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayValue {
    pub element_type: String,
    pub values: Vec<ArrayElement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArrayElement {
    Unimplemented,
}

pub fn read_array_property(
    _r: &mut Reader<'_>,
    _save_version: i32,
    _ue5_version: u32,
    _map_name: &str,
    _parent_type: Option<&str>,
) -> Result<ArrayValue> {
    Err(Error::UnsupportedPropertyType {
        name: String::new(),
        type_name: "Array".to_string(),
        at: 0,
    })
}
