//! `ConveyorBelt` + `ConveyorLift` component. The trailing bytes of a belt/lift
//! actor's entity body contain a count, an items array, and per-item position
//! values along the belt.
//!
//! Wire format (after the entity body's property bag terminates):
//!
//! | Field         | Type            | Notes |
//! |---------------|-----------------|-------|
//! | `count`       | i32             | Unused in JS; passed through. |
//! | `items_length`| i32             | Number of items currently on the belt. |
//! | `items`       | (`InventoryItem` + `position: f32`) × `items_length` |
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:684-698.

#![allow(clippy::derive_partial_eq_without_eq)]

use scim_savefile::{read_inventory_item, EntityBody, InventoryItem, RawActor, Reader};
use serde::{Deserialize, Serialize};

use crate::component::Component;
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConveyorBeltItem {
    pub inventory_item: InventoryItem,
    pub position: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConveyorBelt {
    pub count: i32,
    pub items: Vec<ConveyorBeltItem>,
}

impl Component for ConveyorBelt {
    fn decode(raw_actor: &RawActor<'_>, body: &EntityBody<'_>) -> Result<Self> {
        let mut r = Reader::new(body.trailing_bytes);
        let count = r.read_i32()?;
        let items_length = r.read_i32()?;
        let items_length_usize = usize::try_from(items_length.max(0)).unwrap_or(0);

        // ConveyorBelt items in vanilla saves are simple references; the nested
        // property-bag path inside read_inventory_item only triggers for FIN
        // mod items. Use save_version=46/ue5_version=1000 as safe defaults for
        // our supported range. map_name="" is fine — collapsed-form only fires
        // when the first string matches map_name, which empty string won't.
        let save_version = 46_i32;
        let ue5_version = 1000_u32;
        let map_name = "";

        let mut items = Vec::with_capacity(items_length_usize);
        for _ in 0..items_length_usize {
            let inv = read_inventory_item(&mut r, save_version, ue5_version, map_name)?;
            let position = r.read_f32()?;
            items.push(ConveyorBeltItem {
                inventory_item: inv,
                position,
            });
        }

        if r.remaining() != 0 {
            return Err(Error::ConveyorBeltTrailingBytes {
                bytes_remaining: r.remaining(),
            });
        }

        let _ = raw_actor;
        Ok(Self { count, items })
    }

    fn encode_into(&self, _out: &mut Vec<u8>) -> Result<()> {
        Err(Error::NoComponentForClass {
            class_name: "ConveyorBelt::encode_into (P2)".to_string(),
        })
    }
}
