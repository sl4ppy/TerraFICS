//! `ClassDef` + `ClassKind` — what gameplay role does a given UE class play?
//!
//! Per design spec §4.2, replacing the ad-hoc mod-prefix list in
//! `SC-InteractiveMap/src/Building.js`.

#![allow(clippy::derive_partial_eq_without_eq)]

use serde::{Deserialize, Serialize};

/// Identifier for a mod that contributes class definitions.
/// Vanilla classes have `mod_origin == None`.
pub type ModId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClassKind {
    /// Standard belt (Mk1-Mk6 vanilla, plus mod variants).
    ConveyorBelt,
    /// Lift (vertical belt). Wire-format identical to belt for our purposes.
    ConveyorLift,
    /// Chain-actor container for performance — its trailing bytes have a
    /// different layout than belt/lift. Not decoded in P1.3-c.
    ConveyorChainActor,
    /// Item splitter / merger.
    Splitter,
    /// Resource miner (Mk1-Mk3).
    Miner,
    /// Pipeline (fluid analogue of belt).
    Pipeline,
    /// In-world resource deposit.
    ResourceNode,
    /// Anything not yet categorized — handled generically.
    Unknown,
}

/// Per-class metadata. Future fields (footprint, electrical role, …) layer on top.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassDef {
    pub class_name: String,
    pub kind: ClassKind,
    /// `None` for vanilla classes; `Some(mod_id)` for mod-supplied ones.
    pub mod_origin: Option<ModId>,
}

impl ClassDef {
    #[must_use]
    pub fn vanilla(class_name: impl Into<String>, kind: ClassKind) -> Self {
        Self {
            class_name: class_name.into(),
            kind,
            mod_origin: None,
        }
    }

    #[must_use]
    pub fn from_mod(
        class_name: impl Into<String>,
        kind: ClassKind,
        mod_id: impl Into<ModId>,
    ) -> Self {
        Self {
            class_name: class_name.into(),
            kind,
            mod_origin: Some(mod_id.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_constructor_sets_no_mod_origin() {
        let cd = ClassDef::vanilla("/Game/Foo.Foo_C", ClassKind::ConveyorBelt);
        assert_eq!(cd.kind, ClassKind::ConveyorBelt);
        assert!(cd.mod_origin.is_none());
    }

    #[test]
    fn from_mod_sets_origin() {
        let cd = ClassDef::from_mod("/Mod/Belt", ClassKind::ConveyorBelt, "my-mod");
        assert_eq!(cd.mod_origin.as_deref(), Some("my-mod"));
    }
}
