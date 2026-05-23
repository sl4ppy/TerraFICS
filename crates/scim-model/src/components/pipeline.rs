//! `Pipeline` typed wrapper.
//!
//! Covers vanilla pipelines (Mk1/Mk2), no-indicator variants, and pumps.
//! Property-bag only; the typed view is essentially a class-name discriminator
//! at this stage. Pipeline-specific properties (fluid descriptor, flow rate)
//! sit in nested struct properties and stay accessible via the raw property bag.

#![allow(clippy::derive_partial_eq_without_eq)]

use scim_savefile::{EntityBody, ObjectProperty, RawActor};
use serde::{Deserialize, Serialize};

use crate::component::Component;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineKind {
    Mk1,
    Mk2,
    Mk1NoIndicator,
    Mk2NoIndicator,
    PumpMk1,
    PumpMk2,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pipeline {
    pub entity_reference: Option<ObjectProperty>,
    pub kind: PipelineKind,
}

impl Component for Pipeline {
    fn decode(raw_actor: &RawActor<'_>, body: &EntityBody<'_>) -> Result<Self> {
        let cn = raw_actor.header.class_name.as_str();
        let kind = if cn.contains("PumpMK2") || cn.contains("PumpMk2") {
            PipelineKind::PumpMk2
        } else if cn.contains("Pump") {
            PipelineKind::PumpMk1
        } else if cn.contains("MK2_NoIndicator") {
            PipelineKind::Mk2NoIndicator
        } else if cn.contains("Pipeline_NoIndicator") {
            PipelineKind::Mk1NoIndicator
        } else if cn.contains("MK2") {
            PipelineKind::Mk2
        } else {
            PipelineKind::Mk1
        };
        Ok(Self {
            entity_reference: body.entity_reference.clone(),
            kind,
        })
    }

    fn encode_into(&self, _out: &mut Vec<u8>) -> Result<()> {
        Err(crate::error::Error::NoComponentForClass {
            class_name: "Pipeline::encode_into (P2)".to_string(),
        })
    }
}
