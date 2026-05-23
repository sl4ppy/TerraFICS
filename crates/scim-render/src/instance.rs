//! Per-actor GPU instance data + builder from `scim_world::WorldIndex`.

use scim_world::WorldIndex;

/// One instance entry uploaded to the GPU per actor. Filled in by Task 3.
#[derive(Debug, Default, Clone, Copy)]
pub struct Instance;

/// Walk a `WorldIndex` and emit one `Instance` per `ActorPlacement`.
/// Filled in by Task 4.
#[must_use]
pub const fn build_instances(_index: &WorldIndex) -> Vec<Instance> {
    Vec::new()
}
