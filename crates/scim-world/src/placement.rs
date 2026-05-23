//! Per-actor spatial entry indexed by the R-tree. See `index.rs`.

/// One entry in the world spatial index. Holds the actor's database id and
/// its world-space translation (x, y, z). Indexed in 2D by (x, y); z passes
/// through for caller-side filtering (z-slicing happens in shader per §6.4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorPlacement {
    /// `actor.id` rowid from `scim-store`.
    pub actor_id: i64,
    /// Translation from the actor's transform (`scim_store::decode_transform`).
    pub position: [f32; 3],
}
