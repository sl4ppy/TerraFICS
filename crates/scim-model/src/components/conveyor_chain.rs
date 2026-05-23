//! `ConveyorChainActor` component.
//!
//! Its trailing-bytes layout differs from belt/lift and is substantially more
//! complex: a list of `Conveyor` entries each with a per-segment spline of
//! position + tangent triples, plus an `actual_items` array similar to belts'.
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:700-753.

#![allow(clippy::derive_partial_eq_without_eq)]

use scim_savefile::{
    read_inventory_item, read_object_property, EntityBody, InventoryItem, ObjectProperty, RawActor,
    Reader,
};
use serde::{Deserialize, Serialize};

use crate::component::Component;
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplinePoint {
    pub location: [f64; 3],
    pub arrive_tangent: [f64; 3],
    pub leave_tangent: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainedConveyor {
    pub m_chain_actor: ObjectProperty,
    pub m_conveyor_base: ObjectProperty,
    pub spline_points: Vec<SplinePoint>,
    pub offset_at_start: f32,
    pub starts_at_length: f32,
    pub ends_at_length: f32,
    pub first_item_index: i32,
    pub last_item_index: i32,
    pub index_in_chain_array: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainActorItem {
    pub inventory_item: InventoryItem,
    pub position: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConveyorChainActor {
    pub count: i32,
    pub m_first_conveyor: ObjectProperty,
    pub m_last_conveyor: ObjectProperty,
    pub conveyors: Vec<ChainedConveyor>,
    pub m_total_length: f32,
    pub m_num_items: i32,
    pub m_lead_item_index: i32,
    pub m_tail_item_index: i32,
    pub actual_items: Vec<ChainActorItem>,
}

impl Component for ConveyorChainActor {
    fn decode(raw_actor: &RawActor<'_>, body: &EntityBody<'_>) -> Result<Self> {
        let mut r = Reader::new(body.trailing_bytes);
        let save_version = 46_i32;
        let ue5_version = 1000_u32;
        let map_name = "";

        let count = r.read_i32()?;
        let m_first_conveyor = read_object_property(&mut r, map_name)?;
        let m_last_conveyor = read_object_property(&mut r, map_name)?;
        let m_conveyor_length = r.read_i32()?;
        let m_conveyor_length_usize = usize::try_from(m_conveyor_length.max(0)).unwrap_or(0);

        let mut conveyors = Vec::with_capacity(m_conveyor_length_usize);
        for _ in 0..m_conveyor_length_usize {
            let m_chain_actor = read_object_property(&mut r, map_name)?;
            let m_conveyor_base = read_object_property(&mut r, map_name)?;
            let spline_length = r.read_i32()?;
            let spline_length_usize = usize::try_from(spline_length.max(0)).unwrap_or(0);
            let mut spline_points = Vec::with_capacity(spline_length_usize);
            for _ in 0..spline_length_usize {
                let location = [r.read_f64()?, r.read_f64()?, r.read_f64()?];
                let arrive_tangent = [r.read_f64()?, r.read_f64()?, r.read_f64()?];
                let leave_tangent = [r.read_f64()?, r.read_f64()?, r.read_f64()?];
                spline_points.push(SplinePoint {
                    location,
                    arrive_tangent,
                    leave_tangent,
                });
            }
            let offset_at_start = r.read_f32()?;
            let starts_at_length = r.read_f32()?;
            let ends_at_length = r.read_f32()?;
            let first_item_index = r.read_i32()?;
            let last_item_index = r.read_i32()?;
            let index_in_chain_array = r.read_i32()?;
            conveyors.push(ChainedConveyor {
                m_chain_actor,
                m_conveyor_base,
                spline_points,
                offset_at_start,
                starts_at_length,
                ends_at_length,
                first_item_index,
                last_item_index,
                index_in_chain_array,
            });
        }

        let m_total_length = r.read_f32()?;
        let m_num_items = r.read_i32()?;
        let m_lead_item_index = r.read_i32()?;
        let m_tail_item_index = r.read_i32()?;

        let actual_items_length = r.read_i32()?;
        let actual_items_length_usize = usize::try_from(actual_items_length.max(0)).unwrap_or(0);
        let mut actual_items = Vec::with_capacity(actual_items_length_usize);
        for _ in 0..actual_items_length_usize {
            let inv = read_inventory_item(&mut r, save_version, ue5_version, map_name)?;
            let position = r.read_f32()?;
            actual_items.push(ChainActorItem {
                inventory_item: inv,
                position,
            });
        }

        if r.remaining() != 0 {
            return Err(Error::ConveyorChainActorTrailingBytes {
                bytes_remaining: r.remaining(),
            });
        }

        let _ = raw_actor;
        Ok(Self {
            count,
            m_first_conveyor,
            m_last_conveyor,
            conveyors,
            m_total_length,
            m_num_items,
            m_lead_item_index,
            m_tail_item_index,
            actual_items,
        })
    }

    fn encode_into(&self, _out: &mut Vec<u8>) -> Result<()> {
        Err(Error::NoComponentForClass {
            class_name: "ConveyorChainActor::encode_into (P2)".to_string(),
        })
    }
}
