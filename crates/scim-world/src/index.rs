//! Read-only spatial index over a `scim-store` snapshot. See spec §6.2.

use rstar::RTree;
use rusqlite::Connection;

use crate::error::Result;
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
    /// Materialize an index from a `scim-store` snapshot.
    ///
    /// Reads every actor row attached to `snapshot_id` via
    /// `scim_store::list_actors_in_snapshot`, decodes each transform, and
    /// inserts one `ActorPlacement` per row that has a transform. Rows whose
    /// `transform` column is `NULL` (`ObjectKind::Object`) are skipped — they
    /// have no spatial location.
    pub fn from_snapshot(conn: &Connection, snapshot_id: i64) -> Result<Self> {
        let rows = scim_store::list_actors_in_snapshot(conn, snapshot_id)?;
        let mut placements = Vec::with_capacity(rows.len());
        for row in rows {
            let Some(blob) = row.transform else { continue };
            let (_rot, trans, _scale) = scim_store::decode_transform(&blob)?;
            placements.push(ActorPlacement {
                actor_id: row.id,
                position: trans,
            });
        }
        Ok(Self::from_placements(placements))
    }

    /// Build an index from a pre-collected vector of placements. Uses
    /// `rstar`'s bulk-load constructor, which is `O(n log n)` and produces a
    /// better-balanced tree than incremental insertion.
    #[must_use]
    pub fn from_placements(placements: Vec<ActorPlacement>) -> Self {
        Self {
            tree: RTree::bulk_load(placements),
        }
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

    /// Iterate placements whose `(x, y)` falls within `[min, max]` (inclusive
    /// on both bounds — per `rstar` semantics). Returns references into the
    /// tree; clone if you need owned values.
    pub fn query_aabb(
        &self,
        min: [f32; 2],
        max: [f32; 2],
    ) -> impl Iterator<Item = &ActorPlacement> {
        let env = rstar::AABB::from_corners(min, max);
        self.tree.locate_in_envelope(&env)
    }

    /// Iterate placements whose `(x, y)` equals `point`. Returns all
    /// coincident matches (picking disambiguation happens caller-side).
    pub fn query_point(&self, point: [f32; 2]) -> impl Iterator<Item = &ActorPlacement> {
        let env = rstar::AABB::from_point(point);
        self.tree.locate_in_envelope(&env)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn p(id: i64, x: f32, y: f32) -> ActorPlacement {
        ActorPlacement {
            actor_id: id,
            position: [x, y, 0.0],
        }
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

    #[test]
    fn query_aabb_returns_only_placements_inside() {
        let placements = vec![
            p(1, 0.0, 0.0),
            p(2, 50.0, 50.0),
            p(3, 200.0, 200.0),
            p(4, -300.0, -300.0),
        ];
        let idx = WorldIndex::from_placements(placements);
        let mut hits: Vec<i64> = idx
            .query_aabb([-10.0, -10.0], [100.0, 100.0])
            .map(|pl| pl.actor_id)
            .collect();
        hits.sort_unstable();
        assert_eq!(hits, vec![1, 2]);
    }

    #[test]
    fn query_aabb_empty_when_outside() {
        let idx = WorldIndex::from_placements(vec![p(1, 0.0, 0.0)]);
        assert!(idx
            .query_aabb([100.0, 100.0], [200.0, 200.0])
            .next()
            .is_none());
    }

    #[test]
    fn query_aabb_inclusive_on_boundary() {
        // rstar's AABB envelopes are inclusive on both bounds.
        let idx = WorldIndex::from_placements(vec![p(1, 100.0, 100.0)]);
        assert_eq!(idx.query_aabb([100.0, 100.0], [100.0, 100.0]).count(), 1);
    }

    #[test]
    fn query_point_finds_exact_match() {
        let idx = WorldIndex::from_placements(vec![p(1, 100.0, 100.0), p(2, 200.0, 200.0)]);
        let hits: Vec<i64> = idx
            .query_point([100.0, 100.0])
            .map(|pl| pl.actor_id)
            .collect();
        assert_eq!(hits, vec![1]);
    }

    #[test]
    fn query_point_returns_all_coincident_placements() {
        // Two actors stacked at the same (x, y) — picking should see both.
        let placements = vec![p(1, 50.0, 50.0), p(2, 50.0, 50.0), p(3, 51.0, 50.0)];
        let idx = WorldIndex::from_placements(placements);
        let mut hits: Vec<i64> = idx
            .query_point([50.0, 50.0])
            .map(|pl| pl.actor_id)
            .collect();
        hits.sort_unstable();
        assert_eq!(hits, vec![1, 2]);
    }

    #[test]
    fn query_point_misses_when_no_placement_at_point() {
        let idx = WorldIndex::from_placements(vec![p(1, 0.0, 0.0)]);
        assert!(idx.query_point([10.0, 10.0]).next().is_none());
    }
}

#[cfg(test)]
mod from_snapshot_tests {
    use super::*;
    use scim_store::{
        actor::{encode_transform, insert_actor},
        blob::insert_blob_if_absent,
        snapshot::{add_actor_to_snapshot, create_snapshot},
        Db,
    };

    #[test]
    fn from_snapshot_indexes_actors_with_transforms_and_skips_objects() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn();
        let snapshot_id = create_snapshot(conn, None, 0, "test", None, None).unwrap();

        let hash = insert_blob_if_absent(conn, b"body").unwrap();

        // Actor at (100, 200, 50) with transform.
        let xform = encode_transform([0.0, 0.0, 0.0, 1.0], [100.0, 200.0, 50.0], [1.0, 1.0, 1.0]);
        let a1 = insert_actor(
            conn,
            "Persistent.A1",
            "/Game/A.A_C",
            "Persistent_Level",
            Some(&xform),
            hash,
        )
        .unwrap();
        add_actor_to_snapshot(conn, snapshot_id, a1).unwrap();

        // Actor at (-50, 0, 0).
        let xform2 = encode_transform([0.0, 0.0, 0.0, 1.0], [-50.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let a2 = insert_actor(
            conn,
            "Persistent.A2",
            "/Game/A.A_C",
            "Persistent_Level",
            Some(&xform2),
            hash,
        )
        .unwrap();
        add_actor_to_snapshot(conn, snapshot_id, a2).unwrap();

        // Object-kind row (no transform) — must be skipped.
        let obj = insert_actor(
            conn,
            "Persistent.Obj",
            "/Game/Obj.Obj_C",
            "Persistent_Level",
            None,
            hash,
        )
        .unwrap();
        add_actor_to_snapshot(conn, snapshot_id, obj).unwrap();

        let idx = WorldIndex::from_snapshot(conn, snapshot_id).unwrap();
        assert_eq!(idx.len(), 2, "object-kind row should be skipped");

        let hits: Vec<i64> = idx
            .query_aabb([99.0, 199.0], [101.0, 201.0])
            .map(|p| p.actor_id)
            .collect();
        assert_eq!(hits, vec![a1]);

        let hits2: Vec<i64> = idx
            .query_aabb([-100.0, -100.0], [0.0, 0.0])
            .map(|p| p.actor_id)
            .collect();
        assert_eq!(hits2, vec![a2]);
    }
}
