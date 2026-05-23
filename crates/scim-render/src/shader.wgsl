// Vertex + fragment shader for the scim-render footprint pass.
//
// Layout:
//   Group 0 Binding 0: camera uniform (mat4x4<f32>)
//   Vertex attributes:
//     @location(0) quad_pos: vec2<f32>   — corner of unit quad in [-0.5, 0.5]
//   Instance attributes:
//     @location(1) instance_pos: vec3<f32>  — world translation
//     @location(2) flags: u32               — bit 0 = selected (P1.5-c)
//
// World scale per instance: 100 units (fixed; per-class footprints land
// with scim-assets work). One solid color output, light gray; selected
// actors are tinted (added in P1.5-c Task 2).

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VsIn {
    @location(0) quad_pos: vec2<f32>,
    @location(1) instance_pos: vec3<f32>,
    @location(2) flags: u32,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) @interpolate(flat) flags: u32,
    @location(1) @interpolate(flat) pick_handle: u32,
};

const QUAD_HALF_EXTENT_WORLD: f32 = 50.0; // 100 unit square (±50)
const FLAG_SELECTED: u32 = 1u;

@vertex
fn vs_main(in: VsIn, @builtin(instance_index) instance_idx: u32) -> VsOut {
    let world = vec4<f32>(
        in.instance_pos.x + in.quad_pos.x * QUAD_HALF_EXTENT_WORLD * 2.0,
        in.instance_pos.y + in.quad_pos.y * QUAD_HALF_EXTENT_WORLD * 2.0,
        in.instance_pos.z,
        1.0,
    );
    var out: VsOut;
    out.clip = camera.view_proj * world;
    out.flags = in.flags;
    out.pick_handle = instance_idx + 1u; // 1-based; 0 reserved for "no hit" in the pick pass
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let base = vec4<f32>(0.78, 0.78, 0.82, 1.0);
    let tint = vec4<f32>(1.0, 0.78, 0.20, 1.0); // yellow-orange selection highlight
    if ((in.flags & FLAG_SELECTED) != 0u) {
        return tint;
    }
    return base;
}

// Pick pass: rasterise the same geometry but emit a per-instance handle
// (1-based; 0 reserved for "no hit") into an R32_UINT target. `handle` is
// a reserved word in WGSL so the varying is named `pick_handle`. Consumed
// by PickPass — see picking.rs and design doc 2026-05-23-p1.5c.
@fragment
fn fs_pick(in: VsOut) -> @location(0) vec4<u32> {
    return vec4<u32>(in.pick_handle, 0u, 0u, 1u);
}
