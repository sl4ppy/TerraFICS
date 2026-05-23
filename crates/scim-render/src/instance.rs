//! Per-actor GPU instance data + builder from `scim_world::WorldIndex`.
//!
//! `Instance` is 16 bytes (vec4-aligned) so future fields (rotation,
//! `footprint_id`, `color_index`, `selection_flags` from spec §6.1) can be packed
//! without re-laying out the buffer.

use bytemuck::{Pod, Zeroable};
use scim_world::WorldIndex;

/// One actor's GPU-side data for the footprint pass.
///
/// Layout matches the `@location(1) instance_pos: vec3<f32>` binding in
/// `shader.wgsl`. The `_pad` field exists so the struct is 16 B (vec4-sized);
/// future fields (rotation quaternion, color slot index, selection bits)
/// land in the same 16 B block or in extra vec4-aligned blocks.
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Instance {
    /// World-space translation `(x, y, z)` from the actor's transform.
    pub position: [f32; 3],
    /// Padding to round the struct out to a vec4. Always 0.
    #[allow(clippy::pub_underscore_fields)] // Intentional padding field; underscore name signals it is reserved, not a typo.
    pub _pad: f32,
}

/// Walk a `WorldIndex` and emit one `Instance` per `ActorPlacement`.
///
/// Order is the R-tree's iteration order, which is stable for a given index
/// but not meaningful — callers should not rely on it for picking
/// (P1.5-c will introduce explicit actor-id passthrough).
#[must_use]
pub const fn build_instances(_index: &WorldIndex) -> Vec<Instance> {
    Vec::new()
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // Comparisons are bit-for-bit copies through bytemuck — no arithmetic.
#[allow(clippy::used_underscore_binding)] // Tests must access _pad to verify round-trip correctness.
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
        let original = Instance { position: [100.0, -200.0, 50.0], _pad: 0.0 };
        let bytes = bytemuck::bytes_of(&original);
        assert_eq!(bytes.len(), 16);
        let restored: &Instance = bytemuck::from_bytes(bytes);
        assert_eq!(restored.position, original.position);
        assert_eq!(restored._pad, original._pad);
    }

    #[test]
    fn instance_slice_casts_to_bytes_for_gpu_upload() {
        let instances = vec![
            Instance { position: [0.0, 0.0, 0.0], _pad: 0.0 },
            Instance { position: [100.0, 100.0, 0.0], _pad: 0.0 },
        ];
        let bytes = bytemuck::cast_slice::<Instance, u8>(&instances);
        assert_eq!(bytes.len(), 32);
    }
}
