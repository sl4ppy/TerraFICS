//! `Miner` typed wrapper.
//!
//! Mk1/Mk2/Mk3 — same property layout. Property-bag only; no trailing bytes.
//! Extracts `mExtractResourceNode` (the linked resource node) as a typed reference.

#![allow(clippy::derive_partial_eq_without_eq)]

use scim_savefile::{EntityBody, ObjectProperty, RawActor};
use serde::{Deserialize, Serialize};

use crate::component::Component;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MinerTier {
    Mk1,
    Mk2,
    Mk3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Miner {
    pub entity_reference: Option<ObjectProperty>,
    pub tier: MinerTier,
    /// The resource node this miner is extracting from, if set.
    pub extract_resource_node: Option<ObjectProperty>,
}

impl Component for Miner {
    fn decode(raw_actor: &RawActor<'_>, body: &EntityBody<'_>) -> Result<Self> {
        let class_name = raw_actor.header.class_name.as_str();
        let tier = if class_name.contains("Mk2") || class_name.contains("MK2") {
            MinerTier::Mk2
        } else if class_name.contains("Mk3") || class_name.contains("MK3") {
            MinerTier::Mk3
        } else {
            MinerTier::Mk1
        };

        let extract_resource_node = body
            .find_property("mExtractResourceNode")
            .and_then(|p| p.value.as_object_ref())
            .cloned();

        Ok(Self {
            entity_reference: body.entity_reference.clone(),
            tier,
            extract_resource_node,
        })
    }

    fn encode_into(&self, _out: &mut Vec<u8>) -> Result<()> {
        Err(crate::error::Error::NoComponentForClass {
            class_name: "Miner::encode_into (P2)".to_string(),
        })
    }
}
