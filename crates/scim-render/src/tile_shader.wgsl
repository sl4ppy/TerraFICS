// Vertex + fragment shader for the scim-render base map tile pass (P1.5-d).
//
// Per design spec §6.1. One unit quad is rasterised per resident tile,
// scaled and translated to the tile's world-space AABB. Fragment samples
// the per-tile texture.
//
// Layout:
//   Group 0 Binding 0: camera uniform (mat4x4<f32>) — shared with the
//                      footprint pipeline; same bind group.
//   Group 1 Binding 0: tile texture (2D RGBA8)
//   Group 1 Binding 1: linear sampler
//   Group 1 Binding 2: tile AABB uniform (vec4<f32> = [min_x, min_y, max_x, max_y])
//   Vertex attributes:
//     @location(0) quad_pos: vec2<f32>   — corner of unit quad in [0, 1]

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

struct TileUniform {
    aabb: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var tile_texture: texture_2d<f32>;
@group(1) @binding(1) var tile_sampler: sampler;
@group(1) @binding(2) var<uniform> tile: TileUniform;

struct VsIn {
    @location(0) quad_pos: vec2<f32>,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    let min_x = tile.aabb.x;
    let min_y = tile.aabb.y;
    let max_x = tile.aabb.z;
    let max_y = tile.aabb.w;
    let world_x = mix(min_x, max_x, in.quad_pos.x);
    let world_y = mix(min_y, max_y, in.quad_pos.y);
    var out: VsOut;
    out.clip = camera.view_proj * vec4<f32>(world_x, world_y, 0.0, 1.0);
    // SCIM tile pixel (u=0, v=0) is the NORTH-WEST corner. The unit quad's
    // quad_pos.y=0 is mapped to world `min_y` (see tile_world_aabb's y math
    // for which world-Y direction that means). UV.v = quad_pos.y is the
    // default — adjust tile_world_aabb if the texture comes out mirrored.
    out.uv = vec2<f32>(in.quad_pos.x, in.quad_pos.y);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(tile_texture, tile_sampler, in.uv);
}
