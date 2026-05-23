//! 2D top-down `wgpu` map renderer for `TerraFICS`.
//!
//! Per design spec §6.1: building footprints are drawn as one big instanced
//! pass against a per-actor instance buffer; the spatial source of truth is
//! the `WorldIndex` in `scim-world` (spec §6.2).
//!
//! **P1.5-b scope (this milestone):** wgpu device + surface, 2D orthographic
//! camera, one instanced unit-quad pipeline with a flat color. No picking
//! (P1.5-c), no base tiles (P1.5-d), no splines (P1.5-e), no z-slicing or
//! color palette (P1.5-f). The renderer is driven by a `winit` example
//! binary (`examples/viewer.rs`); the egui shell that will host this in
//! production lives in `scim-app` and lands later.
//!
//! Perf targets (spec §6.6): 60 FPS at 1440p with ~1M actors indexed,
//! ~150k visible. The footprint pass alone is well under budget at the
//! P1.5-a corpus scale (17,974 instances).

pub mod error;
pub use error::{Error, Result};
pub mod camera;
pub use camera::Camera2d;
pub mod instance;
pub use instance::{build_instances, Instance};
pub mod renderer;
pub use renderer::Renderer;

/// The `i64` row id of an actor in `scim-store`. Re-exported so consumers
/// of `scim-render` don't have to pull in `scim-store` to talk about
/// selection.
pub type ActorId = i64;
