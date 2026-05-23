//! Typed game-domain model on top of the byte-level `scim-savefile` parser.
//!
//! Provides:
//! - `ClassDef` / `ClassKind`: classification of UE classes by gameplay role.
//! - `Registry`: HashMap-backed lookup seeded from a built-in vanilla list
//!   (Mk1-Mk6 belts/lifts, all chain-actor variants, splitter family, Mk1-Mk3
//!   miners, Mk1/Mk2 pipelines + pumps, `FGResourceNode` subclasses) and
//!   extensible via TOML mod manifests.
//! - `Component` trait: per-class typed `decode` / `encode_into` / `affected_indices`.
//! - `TypedComponent`: enum unioning all concrete Component impls; obtained
//!   from `Registry::decode_for_actor`.
//! - Concrete components: `ConveyorBelt`, `ConveyorChainActor`, `Splitter`,
//!   `Miner`, `Pipeline`, `ResourceNode`.
//!
//! Roadmap: P2 adds the editable property model and `scim-store` snapshots.

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
