//! Pure-CPU coordinate math for the base map tile layer.
//!
//! End-to-end trace of how SCIM's web map places a tile in game-world space.
//! All identifiers are JS names from `D:\Projects\SC-InteractiveMap\src\GameMap.js`.
//!
//! ```text
//!   // SCIM class fields:
//!   backgroundSize       = 32768
//!   extraBackgroundSize  = 4096
//!   tileSize             = 256
//!   maxTileZoom          = 8
//!   mappingBoundWest     = -324698.832031   // inner game bounds
//!   mappingBoundEast     =  425301.832031
//!   mappingBoundNorth    = -375000.0
//!   mappingBoundSouth    =  375000.0
//!
//!   // start() runs once. Expands the bounds + backgroundSize:
//!   e = (|west| + |east|) / backgroundSize       // ~22.888
//!   t = (|north| + |south|) / backgroundSize     // ~22.888
//!   westOffset  = e * extraBackgroundSize        // ~93750.08
//!   northOffset = t * extraBackgroundSize        //  93750.00
//!   mappingBoundWest  -= westOffset              //  -418448.92
//!   mappingBoundEast  += westOffset              //   519051.92
//!   mappingBoundNorth -= northOffset             //  -468750.0
//!   mappingBoundSouth += northOffset             //   468750.0
//!   backgroundSize    += 2 * extraBackgroundSize //  40960
//!
//!   // zoomRatio(): the Leaflet zoom level at which 1 raster pixel = 1 latlng unit.
//!   zoomRatio = ceil(log2(backgroundSize / tileSize)) = ceil(log2(40960/256))
//!             = ceil(log2(160)) = ceil(7.32) = 8
//!
//!   // convertToRasterCoordinates(game) -> raster:
//!   xMax    = |mappingBoundWest| + |mappingBoundEast|   // 937500.83 (expanded)
//!   yMax    = |mappingBoundNorth| + |mappingBoundSouth| // 937500
//!   xRatio  = backgroundSize / xMax                     // ~0.04369
//!   yRatio  = backgroundSize / yMax                     // ~0.04369
//!   x_raster = (game_x - mappingBoundWest) * xRatio
//!   y_raster = (game_y - mappingBoundNorth) * yRatio
//!
//!   // unproject(raster, zoomRatio) -> Leaflet latlng (CRS.Simple):
//!   scale(z) = tileSize * 2^z = 256 * 2^z
//!   latlng = raster / scale(zoomRatio) = raster / 65536
//!
//!   // Tile (z, x, y) covers, in latlng: [x / 2^z, y / 2^z] to [(x+1)/2^z, (y+1)/2^z].
//!   // Note: pyramid covers full latlng [0, 1], but bounds only cover
//!   // [0, backgroundSize/65536] = [0, 0.625]. Tiles past 0.625 latlng exist in
//!   // the URL space but are outside SCIM's displayed bounds. (Many of those
//!   // are 404 on the CDN since SCIM only generated tiles for the playable area.)
//! ```
//!
//! Putting the chain together to compute tile (z, x, y)'s game-world AABB:
//!
//! ```text
//!   // Tile size in raster pixels at zoom z = 256 * 2^(zoomRatio - z) = 256 * 2^(8 - z)
//!   // Tile size in game units at zoom z = tile_raster_at_z / xRatio
//!   //                                   = (256 * 2^(8-z)) * xMax / backgroundSize
//!   //                                   = (65536 / 2^z) * (937500 / 40960)
//!   //                                   = (1500000 / 2^z) game units per tile
//!   //
//!   // Pyramid NW corner = raster (0, 0) = game (mappingBoundWest, mappingBoundNorth)
//!   //                                   = (-418448.92, -468750)
//! ```
//!
//! So the constants below define the pyramid's NW corner (where tile (z, 0, 0)
//! starts) and the per-axis game-unit size of the FULL pyramid (which is wider
//! than the displayed bounds by a factor of `scale(zoomRatio) / backgroundSize`).

/// Minimum zoom level supplied by the SCIM tile pyramid.
pub const MIN_TILE_ZOOM: u8 = 3;

/// Maximum zoom level supplied by the SCIM tile pyramid.
pub const MAX_TILE_ZOOM: u8 = 8;

/// Pixel dimensions of a single tile (`tileSize` in SCIM JS).
pub const TILE_PIXEL_SIZE: u32 = 256;

/// Game-world coordinate of the western edge of the tile pyramid
/// (= `mappingBoundWest` after `start()` expansion). Tile (z, 0, _) starts
/// here on the X axis.
// reason: derived from -324698.832031 - 93750.0826; f32 precision limit.
#[allow(clippy::excessive_precision)]
pub const TILE_PYRAMID_WEST: f32 = -418_448.92;

/// Game-world coordinate of the northern edge of the tile pyramid
/// (= `mappingBoundNorth` after `start()` expansion). Tile (z, _, 0) starts
/// here on the Y axis (most-negative-Y in SCIM's CRS.Simple convention).
pub const TILE_PYRAMID_NORTH: f32 = -468_750.0;

/// Game-units-per-axis of the FULL tile pyramid (NOT just the displayed
/// bounds). At zoom z, each tile is `TILE_PYRAMID_SIZE / 2^z` game units wide.
///
/// Derived: `scale(zoomRatio) * xMax / backgroundSize`
///        = `65536 * 937500.83 / 40960` ≈ `1_500_001`.
pub const TILE_PYRAMID_SIZE: f32 = 1_500_001.0;

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

/// Game-units-per-tile-axis at the given zoom level.
#[must_use]
pub fn tile_size_game(zoom: u8) -> f32 {
    // reason: 1u32 << 8 = 256, safe for u8 zoom in [3, 8]. f32 cast exact.
    #[allow(clippy::cast_precision_loss)]
    let denom = (1u32 << zoom) as f32;
    TILE_PYRAMID_SIZE / denom
}

/// Pick the tile pyramid zoom such that ~1 tile pixel maps to ~1 screen
/// pixel for the given camera `units_per_pixel`. Clamped to
/// `[MIN_TILE_ZOOM, MAX_TILE_ZOOM]`.
#[must_use]
pub fn tile_zoom_for(units_per_pixel: f32) -> u8 {
    // World units per tile-pixel at zoom z = tile_size_game(z) / TILE_PIXEL_SIZE
    //                                     = (TILE_PYRAMID_SIZE / 2^z) / 256
    // Solve for z when this equals units_per_pixel:
    //   2^z = TILE_PYRAMID_SIZE / (256 * units_per_pixel)
    let upp = units_per_pixel.max(1e-6);
    // reason: TILE_PIXEL_SIZE is a small constant (256); f32 cast is exact.
    #[allow(clippy::cast_precision_loss)]
    let denom = TILE_PIXEL_SIZE as f32 * upp;
    let z = (TILE_PYRAMID_SIZE / denom).log2().round();
    // reason: f32 zoom comes from a clamped log2; conversion to u8 is bounded.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let z_u8 = z.clamp(f32::from(MIN_TILE_ZOOM), f32::from(MAX_TILE_ZOOM)) as u8;
    z_u8
}

/// Enumerate tiles overlapping `camera_world_aabb` at the given `zoom`.
/// AABB is `[min_x, min_y, max_x, max_y]`. Tiles outside the pyramid
/// indices are clamped out.
#[must_use]
pub fn visible_tiles(zoom: u8, camera_world_aabb: [f32; 4]) -> Vec<TileKey> {
    let tiles_per_axis: u32 = 1u32 << zoom;
    let tile_w = tile_size_game(zoom);
    let [min_x, min_y, max_x, max_y] = camera_world_aabb;
    let pyramid_east = TILE_PYRAMID_WEST + TILE_PYRAMID_SIZE;
    let pyramid_south = TILE_PYRAMID_NORTH + TILE_PYRAMID_SIZE;

    // Early-out: AABB entirely off-pyramid.
    if max_x <= TILE_PYRAMID_WEST
        || min_x >= pyramid_east
        || max_y <= TILE_PYRAMID_NORTH
        || min_y >= pyramid_south
    {
        return Vec::new();
    }

    // reason: tiles_per_axis is at most 2^8 = 256; f32 cast is exact.
    #[allow(clippy::cast_precision_loss)]
    let tpa_f = tiles_per_axis as f32;
    let f_to_u32 = |v: f32| -> u32 {
        // reason: v is clamped to [0, tpa_f]; bounded conversion.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let u = v.clamp(0.0, tpa_f) as u32;
        u
    };
    let x0 = f_to_u32(((min_x - TILE_PYRAMID_WEST) / tile_w).floor());
    let x1 = f_to_u32(((max_x - TILE_PYRAMID_WEST) / tile_w).ceil());
    let y0 = f_to_u32(((min_y - TILE_PYRAMID_NORTH) / tile_w).floor());
    let y1 = f_to_u32(((max_y - TILE_PYRAMID_NORTH) / tile_w).ceil());
    let x1 = x1.min(tiles_per_axis);
    let y1 = y1.min(tiles_per_axis);

    let cap = ((x1.saturating_sub(x0)) as usize).saturating_mul((y1.saturating_sub(y0)) as usize);
    let mut out = Vec::with_capacity(cap);
    for y in y0..y1 {
        for x in x0..x1 {
            out.push(TileKey { zoom, x, y });
        }
    }
    out
}

/// World-space AABB `[min_x, min_y, max_x, max_y]` for tile `(zoom, x, y)`.
/// SCIM convention: y=0 covers `[TILE_PYRAMID_NORTH, TILE_PYRAMID_NORTH + tile_h]`
/// (y=0 is the NORTH-most tile row).
#[must_use]
pub fn tile_world_aabb(zoom: u8, x: u32, y: u32) -> [f32; 4] {
    let tile_w = tile_size_game(zoom);
    // reason: x and y are tile indices in [0, 2^zoom <= 256]; f32 cast is exact for u32 < 2^24.
    #[allow(clippy::cast_precision_loss)]
    let xf = x as f32;
    #[allow(clippy::cast_precision_loss)]
    let yf = y as f32;
    let min_x = xf.mul_add(tile_w, TILE_PYRAMID_WEST);
    let min_y = yf.mul_add(tile_w, TILE_PYRAMID_NORTH);
    [min_x, min_y, min_x + tile_w, min_y + tile_w]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn tile_size_game_at_z3_is_about_187500() {
        // Derived: 1_500_001 / 2^3 = 187500.125
        assert!(approx_eq(tile_size_game(3), 187_500.0, 1.0));
    }

    #[test]
    fn tile_size_game_at_z5_is_about_46875() {
        assert!(approx_eq(tile_size_game(5), 46_875.0, 1.0));
    }

    #[test]
    fn tile_zoom_for_default_camera_upp_picks_a_mid_range_zoom() {
        let z = tile_zoom_for(200.0);
        assert!((MIN_TILE_ZOOM..=MAX_TILE_ZOOM).contains(&z), "got z={z}");
    }

    #[test]
    fn tile_zoom_for_very_small_upp_clamps_to_max() {
        assert_eq!(tile_zoom_for(1.0), MAX_TILE_ZOOM);
    }

    #[test]
    fn tile_zoom_for_very_large_upp_clamps_to_min() {
        assert_eq!(tile_zoom_for(50_000.0), MIN_TILE_ZOOM);
    }

    #[test]
    fn visible_tiles_full_pyramid_at_z3_returns_64() {
        let pyramid_east = TILE_PYRAMID_WEST + TILE_PYRAMID_SIZE;
        let pyramid_south = TILE_PYRAMID_NORTH + TILE_PYRAMID_SIZE;
        let aabb = [
            TILE_PYRAMID_WEST,
            TILE_PYRAMID_NORTH,
            pyramid_east,
            pyramid_south,
        ];
        let tiles = visible_tiles(3, aabb);
        assert_eq!(tiles.len(), 64);
    }

    #[test]
    fn visible_tiles_tiny_aabb_at_origin_returns_one_or_two() {
        let aabb = [-0.5, -0.5, 0.5, 0.5];
        let tiles = visible_tiles(5, aabb);
        assert!(
            tiles.len() == 1 || tiles.len() == 2,
            "got {} tiles",
            tiles.len()
        );
    }

    #[test]
    fn visible_tiles_off_pyramid_aabb_returns_empty() {
        let pyramid_east = TILE_PYRAMID_WEST + TILE_PYRAMID_SIZE;
        let aabb = [pyramid_east + 1000.0, 0.0, pyramid_east + 2000.0, 1000.0];
        let tiles = visible_tiles(5, aabb);
        assert!(
            tiles.is_empty(),
            "expected empty, got {} tiles",
            tiles.len()
        );
    }

    #[test]
    fn tile_world_aabb_at_z3_origin_tile_starts_at_pyramid_nw() {
        let aabb = tile_world_aabb(3, 0, 0);
        assert!(approx_eq(aabb[0], TILE_PYRAMID_WEST, 0.01));
        assert!(approx_eq(aabb[1], TILE_PYRAMID_NORTH, 0.01));
        // Size should be ~187500 each axis.
        assert!(approx_eq(aabb[2] - aabb[0], 187_500.0, 1.0));
        assert!(approx_eq(aabb[3] - aabb[1], 187_500.0, 1.0));
    }

    #[test]
    fn tile_world_aabb_contains_space_elevator_actor_in_known_tile() {
        // Regression test: real save (Test_01.sav) has the Space Elevator
        // building at game (-242380, -125005). SCIM's web map places it on
        // tile (3, 0, 1). Verify our math agrees.
        let actor = [-242_380.0_f32, -125_005.0];
        let aabb = tile_world_aabb(3, 0, 1);
        assert!(
            actor[0] >= aabb[0] && actor[0] <= aabb[2],
            "x: actor {} aabb [{}, {}]",
            actor[0],
            aabb[0],
            aabb[2]
        );
        assert!(
            actor[1] >= aabb[1] && actor[1] <= aabb[3],
            "y: actor {} aabb [{}, {}]",
            actor[1],
            aabb[1],
            aabb[3]
        );
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
