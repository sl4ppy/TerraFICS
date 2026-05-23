//! Read-only spatial index over a `scim-store` snapshot. See spec §6.2.

use crate::placement::ActorPlacement;

/// Spatial index over the actors in a snapshot.
#[derive(Debug, Default)]
pub struct WorldIndex {
    placements: Vec<ActorPlacement>,
}

impl WorldIndex {
    /// Number of placements indexed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.placements.len()
    }

    /// Is the index empty?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.placements.is_empty()
    }
}
