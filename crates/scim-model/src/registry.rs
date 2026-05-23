//! `Registry` — lookup `class_name` → `ClassDef`.
//!
//! `Registry::new()` returns the built-in vanilla table seeded from
//! `Building.js`'s class-name lists (P1.3-c covers `ConveyorBelt` /
//! `ConveyorLift` / `ConveyorChainActor`; `Splitter` / `Miner` / `Pipeline` / `ResourceNode`
//! get a small starter set we'll expand in P1.3-d). Mod-defined classes
//! attach via `add_def` or `extend_from_manifests`.

use std::collections::HashMap;

use crate::classdef::{ClassDef, ClassKind};

#[derive(Debug, Clone)]
pub struct Registry {
    by_class_name: HashMap<String, ClassDef>,
}

impl Registry {
    /// New registry seeded with vanilla class defs.
    #[must_use]
    pub fn new() -> Self {
        let mut r = Self {
            by_class_name: HashMap::new(),
        };
        r.seed_vanilla();
        r
    }

    /// Look up a class by its UE class name.
    #[must_use]
    pub fn get(&self, class_name: &str) -> Option<&ClassDef> {
        self.by_class_name.get(class_name)
    }

    /// Classify a class name. Returns `ClassKind::Unknown` if not registered.
    #[must_use]
    pub fn classify(&self, class_name: &str) -> ClassKind {
        self.by_class_name
            .get(class_name)
            .map_or(ClassKind::Unknown, |d| d.kind)
    }

    /// Add (or replace) a class definition. Useful for tests and for loading
    /// from mod manifests at runtime.
    pub fn add_def(&mut self, def: ClassDef) {
        self.by_class_name.insert(def.class_name.clone(), def);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.by_class_name.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_class_name.is_empty()
    }

    fn seed_vanilla(&mut self) {
        for mk in 1..=6 {
            self.add_def(ClassDef::vanilla(
                format!(
                    "/Game/FactoryGame/Buildable/Factory/ConveyorBeltMk{mk}/Build_ConveyorBeltMk{mk}.Build_ConveyorBeltMk{mk}_C"
                ),
                ClassKind::ConveyorBelt,
            ));
        }
        for mk in 1..=6 {
            self.add_def(ClassDef::vanilla(
                format!(
                    "/Game/FactoryGame/Buildable/Factory/ConveyorLiftMk{mk}/Build_ConveyorLiftMk{mk}.Build_ConveyorLiftMk{mk}_C"
                ),
                ClassKind::ConveyorLift,
            ));
        }
        for name in [
            "/Script/FactoryGame.FGConveyorChainActor",
            "/Script/FactoryGame.FGConveyorChainActor_RepSizeMedium",
            "/Script/FactoryGame.FGConveyorChainActor_RepSizeLarge",
            "/Script/FactoryGame.FGConveyorChainActor_RepSizeHuge",
            "/Script/FactoryGame.FGConveyorChainActor_RepSizeNoCull",
        ] {
            self.add_def(ClassDef::vanilla(name, ClassKind::ConveyorChainActor));
        }

        self.add_def(ClassDef::vanilla(
            "/Game/FactoryGame/Buildable/Factory/CA_Splitter/Build_ConveyorAttachmentSplitter.Build_ConveyorAttachmentSplitter_C",
            ClassKind::Splitter,
        ));
        self.add_def(ClassDef::vanilla(
            "/Game/FactoryGame/Buildable/Factory/MinerMK1/Build_MinerMk1.Build_MinerMk1_C",
            ClassKind::Miner,
        ));
        self.add_def(ClassDef::vanilla(
            "/Game/FactoryGame/Buildable/Factory/PipelineMK1/Build_Pipeline.Build_Pipeline_C",
            ClassKind::Pipeline,
        ));
        self.add_def(ClassDef::vanilla(
            "/Script/FactoryGame.FGResourceNode",
            ClassKind::ResourceNode,
        ));
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_seeds_vanilla_conveyors() {
        let r = Registry::new();
        assert!(r.len() >= 21);
        assert_eq!(
            r.classify("/Game/FactoryGame/Buildable/Factory/ConveyorBeltMk3/Build_ConveyorBeltMk3.Build_ConveyorBeltMk3_C"),
            ClassKind::ConveyorBelt
        );
        assert_eq!(
            r.classify("/Game/FactoryGame/Buildable/Factory/ConveyorLiftMk1/Build_ConveyorLiftMk1.Build_ConveyorLiftMk1_C"),
            ClassKind::ConveyorLift
        );
    }

    #[test]
    fn unknown_classes_classify_as_unknown() {
        let r = Registry::new();
        assert_eq!(r.classify("/Some/Random/Thing"), ClassKind::Unknown);
    }

    #[test]
    fn add_def_overrides() {
        let mut r = Registry::new();
        r.add_def(ClassDef::from_mod(
            "/Mod/MyBelt",
            ClassKind::ConveyorBelt,
            "my-mod",
        ));
        let def = r.get("/Mod/MyBelt").expect("should be registered");
        assert_eq!(def.mod_origin.as_deref(), Some("my-mod"));
    }
}
