//! `ResourceNode` typed wrapper.
//!
//! Covers vanilla `FGResourceNode`, `FGResourceDeposit`, and the fracking +
//! geyser subclasses. Property-bag only. Extracts `mResourceClass` (the
//! resource type ref) and `mPurity` (Byte-enum).

#![allow(clippy::derive_partial_eq_without_eq)]

use scim_savefile::{EntityBody, ObjectProperty, RawActor};
use serde::{Deserialize, Serialize};

use crate::component::Component;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceNodeKind {
    Node,
    Deposit,
    FrackingCore,
    FrackingSatellite,
    Geyser,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceNode {
    pub entity_reference: Option<ObjectProperty>,
    pub kind: ResourceNodeKind,
    /// The resource type this node yields, if specified.
    pub resource_class: Option<ObjectProperty>,
    /// Purity tier enum-value name (e.g. `"EResourcePurity::RP_Normal"`), if set.
    pub purity: Option<String>,
}

impl Component for ResourceNode {
    fn decode(raw_actor: &RawActor<'_>, body: &EntityBody<'_>) -> Result<Self> {
        let cn = raw_actor.header.class_name.as_str();
        let kind = if cn.contains("FrackingCore") {
            ResourceNodeKind::FrackingCore
        } else if cn.contains("FrackingSatellite") {
            ResourceNodeKind::FrackingSatellite
        } else if cn.contains("Geyser") {
            ResourceNodeKind::Geyser
        } else if cn.contains("Deposit") {
            ResourceNodeKind::Deposit
        } else {
            ResourceNodeKind::Node
        };

        let resource_class = body
            .find_property("mResourceClass")
            .and_then(|p| p.value.as_object_ref())
            .cloned();
        let purity = body
            .find_property("mPurity")
            .and_then(|p| p.value.as_byte_enum())
            .map(str::to_owned);

        Ok(Self {
            entity_reference: body.entity_reference.clone(),
            kind,
            resource_class,
            purity,
        })
    }

    fn encode_into(&self, _out: &mut Vec<u8>) -> Result<()> {
        Err(crate::error::Error::NoComponentForClass {
            class_name: "ResourceNode::encode_into (P2)".to_string(),
        })
    }
}
