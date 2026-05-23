//! Pure-CPU coordinate math for the base map tile layer.
//!
//! Constants are ported verbatim from `D:\Projects\SC-InteractiveMap\src\GameMap.js`
//! lines 29–36. SCIM's CRS.Simple has y=0 at `WORLD_NORTH` (the most-negative
//! Y). Filled in by Task 2.

/// Minimum zoom level supplied by the SCIM tile pyramid.
pub const MIN_TILE_ZOOM: u8 = 3;

/// Maximum zoom level supplied by the SCIM tile pyramid.
pub const MAX_TILE_ZOOM: u8 = 8;

// reason: value ported verbatim from GameMap.js line 29; f32 can't represent
// all digits but we preserve the literal for source traceability.
#[allow(clippy::excessive_precision)]
/// World coordinate of the western edge of the map.
pub const WORLD_WEST: f32 = -324_698.832_031;

// reason: value ported verbatim from GameMap.js line 30; f32 can't represent
// all digits but we preserve the literal for source traceability.
#[allow(clippy::excessive_precision)]
/// World coordinate of the eastern edge of the map.
pub const WORLD_EAST: f32 = 425_301.832_031;

/// World coordinate of the northern edge of the map (most-negative Y).
pub const WORLD_NORTH: f32 = -375_000.0;

/// World coordinate of the southern edge of the map (most-positive Y).
pub const WORLD_SOUTH: f32 = 375_000.0;

/// Pixel dimensions of a single tile.
pub const TILE_PIXEL_SIZE: u32 = 256;

/// Identifier for a tile in the pyramid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileKey {
    /// Zoom level of the tile.
    pub zoom: u8,
    /// Tile column index.
    pub x: u32,
    /// Tile row index.
    pub y: u32,
}

/// Pick the tile pyramid zoom such that ~1 tile pixel maps to ~1 screen
/// pixel for the given camera `units_per_pixel`. Clamped to
/// `[MIN_TILE_ZOOM, MAX_TILE_ZOOM]`.
#[must_use]
pub fn tile_zoom_for(units_per_pixel: f32) -> u8 {
    let world_w = WORLD_EAST - WORLD_WEST;
    // World units per tile-pixel at zoom z = world_w / (2^z * TILE_PIXEL_SIZE).
    // Solve for z: 2^z = world_w / (TILE_PIXEL_SIZE * units_per_pixel).
    let upp = units_per_pixel.max(1e-6);
    // reason: TILE_PIXEL_SIZE is a small constant (256); f32 cast is exact.
    #[allow(clippy::cast_precision_loss)]
    let denom = TILE_PIXEL_SIZE as f32 * upp;
    let z = (world_w / denom).log2().round();
    // reason: f32 zoom comes from a clamped log2; conversion to u8 is bounded.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let z_u8 = z.clamp(f32::from(MIN_TILE_ZOOM), f32::from(MAX_TILE_ZOOM)) as u8;
    z_u8
}

/// Enumerate tiles overlapping `camera_world_aabb` at the given `zoom`.
/// AABB is `[min_x, min_y, max_x, max_y]`. Tiles fully outside the world
/// bounds are skipped.
#[must_use]
pub fn visible_tiles(zoom: u8, camera_world_aabb: [f32; 4]) -> Vec<TileKey> {
    let tiles_per_axis: u32 = 1u32 << zoom;
    let world_w = WORLD_EAST - WORLD_WEST;
    let world_h = WORLD_SOUTH - WORLD_NORTH;
    // reason: tiles_per_axis is at most 2^8 = 256; f32 cast is exact.
    #[allow(clippy::cast_precision_loss)]
    let tpa_f = tiles_per_axis as f32;
    let tile_w = world_w / tpa_f;
    let tile_h = world_h / tpa_f;
    let [min_x, min_y, max_x, max_y] = camera_world_aabb;

    // Early-out: AABB entirely off-world.
    if max_x <= WORLD_WEST || min_x >= WORLD_EAST || max_y <= WORLD_NORTH || min_y >= WORLD_SOUTH {
        return Vec::new();
    }

    let f_to_u32 = |v: f32| -> u32 {
        // reason: v is clamped to [0, tpa_f]; bounded conversion.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let u = v.clamp(0.0, tpa_f) as u32;
        u
    };
    let x0 = f_to_u32(((min_x - WORLD_WEST) / tile_w).floor());
    let x1 = f_to_u32(((max_x - WORLD_WEST) / tile_w).ceil());
    let y0 = f_to_u32(((min_y - WORLD_NORTH) / tile_h).floor());
    let y1 = f_to_u32(((max_y - WORLD_NORTH) / tile_h).ceil());
    let x1 = x1.min(tiles_per_axis);
    let y1 = y1.min(tiles_per_axis);

    let cap = ((x1.saturating_sub(x0)) as usize)
        .saturating_mul((y1.saturating_sub(y0)) as usize);
    let mut out = Vec::with_capacity(cap);
    for y in y0..y1 {
        for x in x0..x1 {
            out.push(TileKey { zoom, x, y });
        }
    }
    out
}

/// World-space AABB `[min_x, min_y, max_x, max_y]` for tile `(zoom, x, y)`.
/// SCIM convention: y=0 covers `[WORLD_NORTH, WORLD_NORTH + tile_h]`
/// (i.e. y=0 is the NORTH-most tile row).
#[must_use]
pub fn tile_world_aabb(zoom: u8, x: u32, y: u32) -> [f32; 4] {
    let tiles_per_axis: u32 = 1u32 << zoom;
    let world_w = WORLD_EAST - WORLD_WEST;
    let world_h = WORLD_SOUTH - WORLD_NORTH;
    // reason: tiles_per_axis <= 256; f32 conversion is exact.
    #[allow(clippy::cast_precision_loss)]
    let tpa_f = tiles_per_axis as f32;
    let tile_w = world_w / tpa_f;
    let tile_h = world_h / tpa_f;
    // reason: x and y are tile indices in [0, tpa_f]; f32 cast is exact for u32 < 2^24.
    #[allow(clippy::cast_precision_loss)]
    let xf = x as f32;
    #[allow(clippy::cast_precision_loss)]
    let yf = y as f32;
    let min_x = xf.mul_add(tile_w, WORLD_WEST);
    let min_y = yf.mul_add(tile_h, WORLD_NORTH);
    [min_x, min_y, min_x + tile_w, min_y + tile_h]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn tile_zoom_for_units_per_pixel_50_picks_a_mid_range_zoom() {
        let z = tile_zoom_for(50.0);
        assert!((MIN_TILE_ZOOM..=MAX_TILE_ZOOM).contains(&z), "got z={z}");
    }

    #[test]
    fn tile_zoom_for_very_small_upp_clamps_to_max() {
        assert_eq!(tile_zoom_for(1.0), MAX_TILE_ZOOM);
    }

    #[test]
    fn tile_zoom_for_very_large_upp_clamps_to_min() {
        assert_eq!(tile_zoom_for(10_000.0), MIN_TILE_ZOOM);
    }

    #[test]
    fn visible_tiles_full_world_at_z3_returns_64() {
        let aabb = [WORLD_WEST, WORLD_NORTH, WORLD_EAST, WORLD_SOUTH];
        let tiles = visible_tiles(3, aabb);
        assert_eq!(tiles.len(), 64);
    }

    #[test]
    fn visible_tiles_tiny_aabb_at_origin_returns_one_or_two() {
        let aabb = [-0.5, -0.5, 0.5, 0.5];
        let tiles = visible_tiles(5, aabb);
        assert!(tiles.len() == 1 || tiles.len() == 2, "got {} tiles", tiles.len());
    }

    #[test]
    fn visible_tiles_off_world_aabb_returns_empty_or_clamped() {
        let aabb = [WORLD_EAST + 1000.0, 0.0, WORLD_EAST + 2000.0, 1000.0];
        let tiles = visible_tiles(5, aabb);
        assert!(tiles.is_empty(), "expected empty, got {} tiles", tiles.len());
    }

    #[test]
    fn tile_world_aabb_at_z3_origin_tile_covers_a_known_corner() {
        let aabb = tile_world_aabb(3, 0, 0);
        let world_w = WORLD_EAST - WORLD_WEST;
        let world_h = WORLD_SOUTH - WORLD_NORTH;
        assert!(approx_eq(aabb[0], WORLD_WEST, 0.01));
        assert!(approx_eq(aabb[1], WORLD_NORTH, 0.01));
        assert!(approx_eq(aabb[2], WORLD_WEST + world_w / 8.0, 0.01));
        assert!(approx_eq(aabb[3], WORLD_NORTH + world_h / 8.0, 0.01));
    }

    #[test]
    fn tile_world_aabb_round_trips_through_visible_tiles() {
        let camera = [-50_000.0, -50_000.0, 50_000.0, 50_000.0];
        for key in visible_tiles(6, camera) {
            let t = tile_world_aabb(key.zoom, key.x, key.y);
            assert!(
                t[0] < camera[2] && t[2] > camera[0],
                "tile {key:?} aabb {t:?} doesn't overlap camera x"
            );
            assert!(
                t[1] < camera[3] && t[3] > camera[1],
                "tile {key:?} aabb {t:?} doesn't overlap camera y"
            );
        }
    }
}
