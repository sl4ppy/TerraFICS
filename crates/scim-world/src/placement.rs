//! Per-actor spatial entry indexed by the R-tree. See `index.rs`.

use rstar::{RTreeObject, AABB};

/// One entry in the world spatial index.
///
/// Holds the actor's database id and its world-space translation (x, y, z).
/// Indexed in 2D by (x, y); z passes through for caller-side filtering
/// (z-slicing happens in shader per §6.4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorPlacement {
    /// `actor.id` rowid from `scim-store`.
    pub actor_id: i64,
    /// Translation from the actor's transform (`scim_store::decode_transform`).
    pub position: [f32; 3],
}

impl RTreeObject for ActorPlacement {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point([self.position[0], self.position[1]])
    }
}

#[cfg(test)]
// Compares are bit-for-bit copies of the exact same f32 literals — no arithmetic
// or conversion involved, so strict equality is intentional and meaningful.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use rstar::{RTreeObject, AABB};

    #[test]
    fn placement_envelope_is_point_in_xy() {
        let p = ActorPlacement { actor_id: 7, position: [100.0, -50.0, 12.5] };
        let env: AABB<[f32; 2]> = p.envelope();
        // For a point envelope, lower == upper == (x, y); z is ignored.
        assert_eq!(env.lower(), [100.0, -50.0]);
        assert_eq!(env.upper(), [100.0, -50.0]);
    }

    #[test]
    fn placement_envelope_ignores_z() {
        let a = ActorPlacement { actor_id: 1, position: [0.0, 0.0, 999.0] };
        let b = ActorPlacement { actor_id: 2, position: [0.0, 0.0, -999.0] };
        assert_eq!(a.envelope(), b.envelope());
    }
}
