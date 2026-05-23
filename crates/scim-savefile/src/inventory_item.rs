//! `InventoryItem` — one item entry inside an inventory or on a conveyor belt.
//!
//! For `save_version >= 41` (our supported minimum) and per-entity `save_version`
//! `>= 44` (covers all observed saves in our corpus): the wire format is
//! `item_name (ObjectProperty) + item_state_flag (i32) +
//!  [if flag != 0: item_state (ObjectProperty) + body]`.
//!
//! Body cases when flag != 0:
//! - `item_state.path_name == "/Script/FicsItNetworksComputer.FINItemStateFileSystem"`:
//!   `i32 hex_length + hex_length bytes` captured opaquely.
//! - Otherwise: `i32 properties_length + property bag (read until None sentinel)`.
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:2605-2643.

#![allow(clippy::derive_partial_eq_without_eq)]

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::object_property::{read_object_property, ObjectProperty};
use crate::property::{read_properties, Property};
use crate::reader::Reader;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryItem {
    pub item_name: ObjectProperty,
    /// `None` when `item_state_flag == 0` (item has no state).
    pub item_state: Option<ObjectProperty>,
    pub state_body: InventoryItemStateBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InventoryItemStateBody {
    /// No state body — either `item_state_flag` was 0 OR `item_state` was set but
    /// there were no nested properties.
    None,
    /// Standard property-bag body.
    Properties(Vec<Property>),
    /// FicsIt-Networks file-system state — captured as opaque bytes.
    FinFileSystem(Vec<u8>),
}

/// Read one inventory-item record.
///
/// `save_version`, `ue5_version`, and `map_name` are passed through to nested
/// property reading (state bodies that contain properties recurse into the
/// existing dispatcher).
pub fn read_inventory_item(
    r: &mut Reader<'_>,
    save_version: i32,
    ue5_version: u32,
    map_name: &str,
) -> Result<InventoryItem> {
    let item_name = read_object_property(r, map_name)?;

    let item_state_flag = r.read_i32()?;
    if item_state_flag == 0 {
        return Ok(InventoryItem {
            item_name,
            item_state: None,
            state_body: InventoryItemStateBody::None,
        });
    }

    let item_state = read_object_property(r, map_name)?;

    let state_body = if item_state.path_name
        == "/Script/FicsItNetworksComputer.FINItemStateFileSystem"
    {
        let hex_length = r.read_i32()?;
        let hex_length_usize = usize::try_from(hex_length.max(0)).unwrap_or(0);
        let bytes = r.read_hex(hex_length_usize)?;
        InventoryItemStateBody::FinFileSystem(bytes)
    } else {
        let _properties_length = r.read_i32()?;
        let bag = read_properties(r, save_version, ue5_version, map_name, Some("InventoryItem"))?;
        InventoryItemStateBody::Properties(bag.properties)
    };

    Ok(InventoryItem {
        item_name,
        item_state: Some(item_state),
        state_body,
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
    fn decodes_simple_item_no_state() {
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "Persistent_Level");
        write_ascii(&mut bytes, "Desc_IronIngot");
        bytes.extend_from_slice(&0_i32.to_le_bytes());
        let mut r = Reader::new(&bytes);
        let item = read_inventory_item(&mut r, 46, 1000, "MapName").unwrap();
        assert_eq!(item.item_name.path_name, "Desc_IronIngot");
        assert!(item.item_state.is_none());
        assert!(matches!(item.state_body, InventoryItemStateBody::None));
    }

    #[test]
    fn decodes_item_with_property_state() {
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "Persistent_Level");
        write_ascii(&mut bytes, "Desc_Something");
        bytes.extend_from_slice(&1_i32.to_le_bytes());
        write_ascii(&mut bytes, "Persistent_Level");
        write_ascii(&mut bytes, "State_Something_42");
        bytes.extend_from_slice(&0_i32.to_le_bytes());
        write_ascii(&mut bytes, "None");
        let mut r = Reader::new(&bytes);
        let item = read_inventory_item(&mut r, 46, 1000, "MapName").unwrap();
        assert!(item.item_state.is_some());
        match item.state_body {
            InventoryItemStateBody::Properties(props) => assert_eq!(props.len(), 0),
            other => panic!("expected Properties, got {other:?}"),
        }
    }
}
