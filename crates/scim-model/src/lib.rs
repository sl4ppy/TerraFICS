//! Typed game-domain model on top of the byte-level `scim-savefile` parser.
//!
//! Capability lands incrementally across P1.3-c tasks:
//! - Task 3: `Error` / `Result`
//! - Task 4: `ClassDef`, `ClassKind`, `ModId`
//! - Task 5: `Registry`
//! - Task 6: `Component` trait
//! - Task 7: `ConveyorBelt` reference component
//! - Task 8: `ModManifest` + TOML loader

pub mod error;
pub use error::{Error, Result};
pub mod classdef;
pub use classdef::{ClassDef, ClassKind, ModId};
pub mod component;
pub use component::Component;
pub mod manifest;
pub use manifest::{load_manifest, load_manifests_from_dir, ModManifest, ModManifestEntry};
pub mod registry;
pub use registry::{Registry, TypedComponent};
pub mod components;
pub use components::{
    ChainActorItem, ChainedConveyor, ConveyorBelt, ConveyorBeltItem, ConveyorChainActor, Miner,
    MinerTier, Pipeline, PipelineKind, ResourceNode, ResourceNodeKind, SplinePoint, Splitter,
    SplitterKind,
};
