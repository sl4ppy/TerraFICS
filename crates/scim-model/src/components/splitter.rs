//! `Splitter` / `Merger` typed wrapper.
//!
//! Property-bag only; no trailing bytes. Covers basic splitter, smart splitter,
//! programmable splitter, and merger. P1.3-d captures only the entity reference
//! and a class-name discriminator; smart-splitter routing rules live in nested
//! struct properties (deferred).

#![allow(clippy::derive_partial_eq_without_eq)]

use scim_savefile::{EntityBody, ObjectProperty, RawActor};
use serde::{Deserialize, Serialize};

use crate::component::Component;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitterKind {
    Basic,
    Merger,
    Smart,
    Programmable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Splitter {
    pub entity_reference: Option<ObjectProperty>,
    pub kind: SplitterKind,
}

impl Component for Splitter {
    fn decode(raw_actor: &RawActor<'_>, body: &EntityBody<'_>) -> Result<Self> {
        let class_name = raw_actor.header.class_name.as_str();
        let kind = if class_name.contains("Smart") {
            SplitterKind::Smart
        } else if class_name.contains("Programmable") {
            SplitterKind::Programmable
        } else if class_name.contains("Merger") {
            SplitterKind::Merger
        } else {
            SplitterKind::Basic
        };
        Ok(Self {
            entity_reference: body.entity_reference.clone(),
            kind,
        })
    }

    fn encode_into(&self, _out: &mut Vec<u8>) -> Result<()> {
        Err(crate::error::Error::NoComponentForClass {
            class_name: "Splitter::encode_into (P2)".to_string(),
        })
    }
}
