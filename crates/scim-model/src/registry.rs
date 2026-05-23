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

    /// Extend the registry with class definitions from a directory of TOML mod
    /// manifests. Returns the number of class defs added.
    pub fn extend_from_manifests(&mut self, dir: &std::path::Path) -> crate::error::Result<usize> {
        let manifests = crate::manifest::load_manifests_from_dir(dir)?;
        let mut added = 0_usize;
        for m in manifests {
            for def in m.to_class_defs() {
                self.add_def(def);
                added += 1;
            }
        }
        Ok(added)
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
        // ConveyorBelt Mk1-Mk6.
        for mk in 1..=6 {
            self.add_def(ClassDef::vanilla(
                format!(
                    "/Game/FactoryGame/Buildable/Factory/ConveyorBeltMk{mk}/Build_ConveyorBeltMk{mk}.Build_ConveyorBeltMk{mk}_C"
                ),
                ClassKind::ConveyorBelt,
            ));
        }
        // ConveyorLift Mk1-Mk6.
        for mk in 1..=6 {
            self.add_def(ClassDef::vanilla(
                format!(
                    "/Game/FactoryGame/Buildable/Factory/ConveyorLiftMk{mk}/Build_ConveyorLiftMk{mk}.Build_ConveyorLiftMk{mk}_C"
                ),
                ClassKind::ConveyorLift,
            ));
        }
        // ConveyorChainActor variants.
        for name in [
            "/Script/FactoryGame.FGConveyorChainActor",
            "/Script/FactoryGame.FGConveyorChainActor_RepSizeMedium",
            "/Script/FactoryGame.FGConveyorChainActor_RepSizeLarge",
            "/Script/FactoryGame.FGConveyorChainActor_RepSizeHuge",
            "/Script/FactoryGame.FGConveyorChainActor_RepSizeNoCull",
        ] {
            self.add_def(ClassDef::vanilla(name, ClassKind::ConveyorChainActor));
        }

        // Splitter family: basic, merger, smart, programmable.
        for name in [
            "/Game/FactoryGame/Buildable/Factory/CA_Splitter/Build_ConveyorAttachmentSplitter.Build_ConveyorAttachmentSplitter_C",
            "/Game/FactoryGame/Buildable/Factory/CA_Merger/Build_ConveyorAttachmentMerger.Build_ConveyorAttachmentMerger_C",
            "/Game/FactoryGame/Buildable/Factory/CA_SplitterSmart/Build_ConveyorAttachmentSplitterSmart.Build_ConveyorAttachmentSplitterSmart_C",
            "/Game/FactoryGame/Buildable/Factory/CA_SplitterProgrammable/Build_ConveyorAttachmentSplitterProgrammable.Build_ConveyorAttachmentSplitterProgrammable_C",
        ] {
            self.add_def(ClassDef::vanilla(name, ClassKind::Splitter));
        }

        // Miner Mk1-Mk3.
        self.add_def(ClassDef::vanilla(
            "/Game/FactoryGame/Buildable/Factory/MinerMK1/Build_MinerMk1.Build_MinerMk1_C",
            ClassKind::Miner,
        ));
        self.add_def(ClassDef::vanilla(
            "/Game/FactoryGame/Buildable/Factory/MinerMk2/Build_MinerMk2.Build_MinerMk2_C",
            ClassKind::Miner,
        ));
        self.add_def(ClassDef::vanilla(
            "/Game/FactoryGame/Buildable/Factory/MinerMk3/Build_MinerMk3.Build_MinerMk3_C",
            ClassKind::Miner,
        ));

        // Pipeline family.
        for name in [
            "/Game/FactoryGame/Buildable/Factory/Pipeline/Build_Pipeline.Build_Pipeline_C",
            "/Game/FactoryGame/Buildable/Factory/PipelineMk2/Build_PipelineMK2.Build_PipelineMK2_C",
            "/Game/FactoryGame/Buildable/Factory/Pipeline/Build_Pipeline_NoIndicator.Build_Pipeline_NoIndicator_C",
            "/Game/FactoryGame/Buildable/Factory/PipelineMk2/Build_PipelineMK2_NoIndicator.Build_PipelineMK2_NoIndicator_C",
            "/Game/FactoryGame/Buildable/Factory/PipePump/Build_PipelinePump.Build_PipelinePump_C",
            "/Game/FactoryGame/Buildable/Factory/PipePumpMk2/Build_PipelinePumpMK2.Build_PipelinePumpMk2_C",
        ] {
            self.add_def(ClassDef::vanilla(name, ClassKind::Pipeline));
        }

        // ResourceNode subclasses.
        for name in [
            "/Script/FactoryGame.FGResourceNode",
            "/Script/FactoryGame.FGResourceDeposit",
            "/Script/FactoryGame.FGResourceNodeFrackingCore",
            "/Script/FactoryGame.FGResourceNodeFrackingSatellite",
            "/Script/FactoryGame.FGResourceNodeGeyser",
        ] {
            self.add_def(ClassDef::vanilla(name, ClassKind::ResourceNode));
        }
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
        assert!(r.len() >= 30, "expected expanded seed; got {}", r.len());
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

    #[test]
    fn extends_from_manifests_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.toml"),
            r#"mod_id = "x"

[[classes]]
class_name = "/X/Belt"
kind = "ConveyorBelt"
"#,
        )
        .unwrap();
        let mut r = Registry::new();
        let added = r.extend_from_manifests(dir.path()).unwrap();
        assert_eq!(added, 1);
        assert_eq!(r.classify("/X/Belt"), ClassKind::ConveyorBelt);
    }
}
