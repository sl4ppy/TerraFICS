//! Per-actor GPU instance data + builder from `scim_world::WorldIndex`.
//!
//! `Instance` is 16 bytes (vec4-aligned) so future fields (rotation,
//! `footprint_id`, `color_index`, `selection_flags` from spec §6.1) can be packed
//! without re-laying out the buffer.

use bytemuck::{Pod, Zeroable};
use scim_world::WorldIndex;

/// One actor's GPU-side data for the footprint pass.
///
/// Layout matches the `@location(1) instance_pos: vec3<f32>` +
/// `@location(2) flags: u32` bindings in `shader.wgsl`. The struct is 16 B
/// (vec4-sized) so future fields (rotation quaternion, footprint id, color
/// slot) can be packed without re-laying out the buffer.
#[derive(Debug, Default, Clone, Copy, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct Instance {
    /// World-space translation `(x, y, z)` from the actor's transform.
    pub position: [f32; 3],
    /// Bit flags. `Instance::FLAG_SELECTED` (bit 0) is the only one defined
    /// in P1.5-c. Future bits land with P1.5-f (palette / per-actor colour
    /// slot) and scim-assets work (footprint id).
    pub flags: u32,
}

impl Instance {
    /// Bit flag indicating this instance is currently selected; the main
    /// fragment shader tints it.
    pub const FLAG_SELECTED: u32 = 1;
}

/// Walk a `WorldIndex` and emit one `Instance` per `ActorPlacement`.
///
/// Order is the R-tree's iteration order, which is stable for a given index
/// but not meaningful — callers should not rely on it for picking
/// (P1.5-c will introduce explicit actor-id passthrough).
#[must_use]
pub fn build_instances(index: &WorldIndex) -> Vec<Instance> {
    index
        .iter()
        .map(|placement| Instance {
            position: placement.position,
            flags: 0,
        })
        .collect()
}

#[cfg(test)]
// reason: position arrays are bit-for-bit identical through bytemuck round-trips and Vec sorts — no arithmetic involved.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn instance_is_16_bytes() {
        assert_eq!(std::mem::size_of::<Instance>(), 16);
    }

    #[test]
    fn instance_alignment_is_at_most_4() {
        // wgpu requires vertex/instance attributes to be at most 4-byte aligned
        // for f32x3/f32x4 attributes — Pod stays valid at align 4.
        assert!(std::mem::align_of::<Instance>() <= 4);
    }

    #[test]
    fn instance_round_trips_through_bytemuck_cast() {
        let original = Instance {
            position: [100.0, -200.0, 50.0],
            flags: 0,
        };
        let bytes = bytemuck::bytes_of(&original);
        assert_eq!(bytes.len(), 16);
        let restored: &Instance = bytemuck::from_bytes(bytes);
        assert_eq!(restored.position, original.position);
        assert_eq!(restored.flags, original.flags);
    }

    #[test]
    fn instance_slice_casts_to_bytes_for_gpu_upload() {
        let instances = vec![
            Instance {
                position: [0.0, 0.0, 0.0],
                flags: 0,
            },
            Instance {
                position: [100.0, 100.0, 0.0],
                flags: 0,
            },
        ];
        let bytes = bytemuck::cast_slice::<Instance, u8>(&instances);
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn build_instances_emits_one_per_placement() {
        use scim_world::{ActorPlacement, WorldIndex};

        let placements = vec![
            ActorPlacement {
                actor_id: 1,
                position: [10.0, 20.0, 30.0],
            },
            ActorPlacement {
                actor_id: 2,
                position: [-5.0, 0.0, 0.0],
            },
        ];
        let idx = WorldIndex::from_placements(placements);

        let instances = build_instances(&idx);
        assert_eq!(instances.len(), 2);
        // R-tree iteration order isn't position order — sort by x to check.
        let mut by_x: Vec<&Instance> = instances.iter().collect();
        by_x.sort_by(|a, b| a.position[0].partial_cmp(&b.position[0]).unwrap());
        assert_eq!(by_x[0].position, [-5.0, 0.0, 0.0]);
        assert_eq!(by_x[1].position, [10.0, 20.0, 30.0]);
    }

    #[test]
    fn build_instances_empty_when_index_empty() {
        use scim_world::WorldIndex;

        let idx = WorldIndex::from_placements(Vec::new());
        let instances = build_instances(&idx);
        assert!(instances.is_empty());
    }

    #[test]
    fn instance_has_flags_field_and_selected_constant() {
        let i = Instance {
            position: [0.0, 0.0, 0.0],
            flags: Instance::FLAG_SELECTED,
        };
        assert_eq!(i.flags & Instance::FLAG_SELECTED, Instance::FLAG_SELECTED);
        assert_eq!(Instance::FLAG_SELECTED, 1);
    }

    #[test]
    fn instance_flags_round_trip_through_bytemuck() {
        let original = Instance {
            position: [1.0, 2.0, 3.0],
            flags: Instance::FLAG_SELECTED,
        };
        let bytes = bytemuck::bytes_of(&original);
        let restored: &Instance = bytemuck::from_bytes(bytes);
        assert_eq!(restored.position, original.position);
        assert_eq!(restored.flags, Instance::FLAG_SELECTED);
    }
}
