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
};

const QUAD_HALF_EXTENT_WORLD: f32 = 50.0; // 100 unit square (±50)

@vertex
fn vs_main(in: VsIn) -> VsOut {
    let world = vec4<f32>(
        in.instance_pos.x + in.quad_pos.x * QUAD_HALF_EXTENT_WORLD * 2.0,
        in.instance_pos.y + in.quad_pos.y * QUAD_HALF_EXTENT_WORLD * 2.0,
        in.instance_pos.z,
        1.0,
    );
    var out: VsOut;
    out.clip = camera.view_proj * world;
    return out;
}

@fragment
fn fs_main(_in: VsOut) -> @location(0) vec4<f32> {
    // Light gray — placeholder until P1.5-f's palette UBO lands.
    return vec4<f32>(0.78, 0.78, 0.82, 1.0);
}
