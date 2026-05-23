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
pub mod registry;
pub use registry::Registry;
