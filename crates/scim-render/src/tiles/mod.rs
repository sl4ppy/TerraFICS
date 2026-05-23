//! Base map tile layer for the scim-render footprint viewer (P1.5-d).
//!
//! Per design spec §6.1 and the P1.5-d MVP design doc: PNG tiles laid out
//! as `{root}/{z}/{x}/{y}.png` are loaded on a background thread and
//! rendered as textured quads underneath the actor footprints. Dynamic
//! zoom selection by `Camera2d::units_per_pixel`. No network / no CDN
//! fetch in this milestone — see the spec for the follow-up plan.

pub mod coord;
pub mod loader;

/// Off-screen tile pass — pipeline + tile cache + loader handle.
/// Constructed by `Renderer` when `set_tile_root(Some(_))` is called.
#[derive(Debug)]
pub struct TilePass {
    // Fields filled in by Task 4+.
    _placeholder: (),
}

impl TilePass {
    /// Construct a tile pass against the given root directory. Filled in by
    /// Task 4.
    #[must_use]
    pub const fn new() -> Self {
        Self { _placeholder: () }
    }
}

impl Default for TilePass {
    fn default() -> Self {
        Self::new()
    }
}
