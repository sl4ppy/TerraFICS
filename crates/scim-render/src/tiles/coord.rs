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
