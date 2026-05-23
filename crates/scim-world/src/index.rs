//! Read-only spatial index over a `scim-store` snapshot. See spec §6.2.

use rstar::RTree;

use crate::placement::ActorPlacement;

/// Spatial index over the actors in a snapshot.
///
/// Construction: see `WorldIndex::from_placements` (bulk-load) or
/// `WorldIndex::from_snapshot` (read directly from a `scim-store` connection).
#[derive(Debug)]
pub struct WorldIndex {
    tree: RTree<ActorPlacement>,
}

impl WorldIndex {
    /// Build an index from a pre-collected vector of placements. Uses
    /// `rstar`'s bulk-load constructor, which is `O(n log n)` and produces a
    /// better-balanced tree than incremental insertion.
    #[must_use]
    pub fn from_placements(placements: Vec<ActorPlacement>) -> Self {
        Self { tree: RTree::bulk_load(placements) }
    }

    /// Number of placements indexed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tree.size()
    }

    /// Is the index empty?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tree.size() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn p(id: i64, x: f32, y: f32) -> ActorPlacement {
        ActorPlacement { actor_id: id, position: [x, y, 0.0] }
    }

    #[test]
    fn empty_index_is_empty() {
        let idx = WorldIndex::from_placements(Vec::new());
        assert!(idx.is_empty());
        assert_eq!(idx.len(), 0);
    }

    #[test]
    fn from_placements_preserves_count() {
        let placements = vec![p(1, 0.0, 0.0), p(2, 100.0, 100.0), p(3, -50.0, 50.0)];
        let idx = WorldIndex::from_placements(placements);
        assert_eq!(idx.len(), 3);
        assert!(!idx.is_empty());
    }
}
