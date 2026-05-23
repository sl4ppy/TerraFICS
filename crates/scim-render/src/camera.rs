//! 2D orthographic top-down camera over world (x, y) space.
//!
//! Per design spec §6: the renderer is 2D top-down; height stacking is
//! handled later by a vertex-shader z-filter (P1.5-f). The camera lives
//! in CPU code and uploads a single view-projection matrix per frame.

use glam::{Mat4, Vec3};

/// 2D orthographic camera. Looks down +Z, with +Y up on screen.
#[derive(Debug, Clone, Copy)]
pub struct Camera2d {
    center: [f32; 2],
    units_per_pixel: f32,
    viewport: [u32; 2],
}

impl Camera2d {
    /// New camera centered at world origin, fitted to the given viewport
    /// with a default zoom of 50 world units per pixel (gives a ~96 km
    /// wide view at 1920×1080 — comfortable for full-map overview).
    #[must_use]
    pub fn new(viewport: [u32; 2]) -> Self {
        Self::with_params([0.0, 0.0], 50.0, viewport)
    }

    /// New camera with explicit parameters.
    #[must_use]
    pub fn with_params(center: [f32; 2], units_per_pixel: f32, viewport: [u32; 2]) -> Self {
        Self {
            center,
            units_per_pixel: units_per_pixel.max(1e-6),
            viewport,
        }
    }

    #[must_use]
    pub const fn center(&self) -> [f32; 2] {
        self.center
    }

    #[must_use]
    pub const fn units_per_pixel(&self) -> f32 {
        self.units_per_pixel
    }

    #[must_use]
    pub const fn viewport(&self) -> [u32; 2] {
        self.viewport
    }

    /// Resize the camera's viewport (call from `WindowEvent::Resized`).
    pub fn resize(&mut self, viewport: [u32; 2]) {
        self.viewport = viewport;
    }

    /// Shift the camera center by a world-space delta.
    pub fn pan(&mut self, delta_world: [f32; 2]) {
        self.center[0] += delta_world[0];
        self.center[1] += delta_world[1];
    }

    /// Zoom by `factor` (>1 = zoom in, <1 = zoom out), keeping the world
    /// point under `screen_xy` invariant on screen.
    pub fn zoom_at(&mut self, factor: f32, screen_xy: [f32; 2]) {
        let factor = factor.max(1e-6);
        let before_world = self.world_from_screen(screen_xy);
        self.units_per_pixel = (self.units_per_pixel / factor).max(1e-6);
        let after_world = self.world_from_screen(screen_xy);
        self.center[0] += before_world[0] - after_world[0];
        self.center[1] += before_world[1] - after_world[1];
    }

    /// Convert a screen-pixel point (origin = top-left, +Y = down) to a
    /// world-space (x, y).
    #[must_use]
    pub fn world_from_screen(&self, screen_xy: [f32; 2]) -> [f32; 2] {
        // Loss of precision when viewport > 16M pixels — not a real workload.
        #[allow(clippy::cast_precision_loss)]
        let half_width = self.viewport[0] as f32 * 0.5;
        #[allow(clippy::cast_precision_loss)]
        let half_height = self.viewport[1] as f32 * 0.5;
        let offset_x = screen_xy[0] - half_width;
        // Flip Y so that +Y on screen (down) becomes -Y in world space.
        let offset_y = screen_xy[1] - half_height;
        [
            offset_x.mul_add(self.units_per_pixel, self.center[0]),
            (-offset_y).mul_add(self.units_per_pixel, self.center[1]),
        ]
    }

    /// World-space AABB `[min_x, min_y, max_x, max_y]` of the current
    /// viewport. Computed from `center`, `viewport`, and `units_per_pixel`.
    #[must_use]
    pub fn world_aabb(&self) -> [f32; 4] {
        // reason: viewport dimensions are window pixel counts; well below 2^24.
        #[allow(clippy::cast_precision_loss)]
        let half_w = self.viewport[0] as f32 * 0.5 * self.units_per_pixel;
        // reason: viewport dimensions are window pixel counts; well below 2^24.
        #[allow(clippy::cast_precision_loss)]
        let half_h = self.viewport[1] as f32 * 0.5 * self.units_per_pixel;
        [
            self.center[0] - half_w,
            self.center[1] - half_h,
            self.center[0] + half_w,
            self.center[1] + half_h,
        ]
    }

    /// View-projection matrix (column-major 4x4) for uploading to a wgpu
    /// uniform buffer. Maps world (x, y, z) -> clip space.
    #[must_use]
    pub fn view_proj(&self) -> [[f32; 4]; 4] {
        // Loss of precision when viewport > 16M pixels — not a real workload.
        #[allow(clippy::cast_precision_loss)]
        let half_width_world = self.viewport[0] as f32 * 0.5 * self.units_per_pixel;
        #[allow(clippy::cast_precision_loss)]
        let half_height_world = self.viewport[1] as f32 * 0.5 * self.units_per_pixel;
        // Orthographic projection (right-handed, +Z forward = into screen).
        let proj = Mat4::orthographic_rh(
            -half_width_world,
            half_width_world,
            -half_height_world,
            half_height_world,
            -1.0e6,
            1.0e6,
        );
        let view = Mat4::from_translation(Vec3::new(-self.center[0], -self.center[1], 0.0));
        (proj * view).to_cols_array_2d()
    }
}

impl Default for Camera2d {
    fn default() -> Self {
        Self::new([1, 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    fn mat_eq(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> bool {
        a.iter().zip(b.iter()).all(|(ra, rb)| {
            ra.iter()
                .zip(rb.iter())
                .all(|(x, y)| approx_eq(*x, *y, 1e-4))
        })
    }

    #[test]
    fn camera_new_centers_at_origin() {
        let cam = Camera2d::new([1920, 1080]);
        assert!(approx_eq(cam.center()[0], 0.0, 1e-6));
        assert!(approx_eq(cam.center()[1], 0.0, 1e-6));
        assert!(cam.units_per_pixel() > 0.0);
        assert_eq!(cam.viewport(), [1920, 1080]);
    }

    #[test]
    fn camera_pan_shifts_center() {
        let mut cam = Camera2d::new([1920, 1080]);
        cam.pan([100.0, -50.0]);
        assert!(approx_eq(cam.center()[0], 100.0, 1e-4));
        assert!(approx_eq(cam.center()[1], -50.0, 1e-4));
        cam.pan([10.0, 10.0]);
        assert!(approx_eq(cam.center()[0], 110.0, 1e-4));
        assert!(approx_eq(cam.center()[1], -40.0, 1e-4));
    }

    #[test]
    fn camera_zoom_at_keeps_anchor_world_position_invariant() {
        // Zooming should leave the world-space point under the cursor unmoved
        // on screen — the canonical zoom-at-cursor invariant.
        let mut cam = Camera2d::new([1000, 1000]);
        let anchor_screen = [200.0_f32, 800.0];
        let world_before = cam.world_from_screen(anchor_screen);
        cam.zoom_at(2.0, anchor_screen);
        let world_after = cam.world_from_screen(anchor_screen);
        assert!(approx_eq(world_before[0], world_after[0], 1e-3));
        assert!(approx_eq(world_before[1], world_after[1], 1e-3));
    }

    #[test]
    fn world_aabb_centered_at_origin_with_unit_upp() {
        let cam = Camera2d::with_params([0.0, 0.0], 1.0, [200, 100]);
        let aabb = cam.world_aabb();
        assert!(approx_eq(aabb[0], -100.0, 1e-4));
        assert!(approx_eq(aabb[1], -50.0, 1e-4));
        assert!(approx_eq(aabb[2], 100.0, 1e-4));
        assert!(approx_eq(aabb[3], 50.0, 1e-4));
    }

    #[test]
    fn camera_view_proj_at_origin_with_unit_upp_matches_ortho() {
        // With center=(0,0), upp=1, viewport=2x2: world (0,0,0) maps to NDC (0,0).
        let cam = Camera2d::with_params([0.0, 0.0], 1.0, [2, 2]);
        let vp = cam.view_proj();
        let p = [0.0_f32, 0.0, 0.0, 1.0];
        let r = mul_mat4_vec4(&vp, p);
        assert!(approx_eq(r[0] / r[3], 0.0, 1e-4));
        assert!(approx_eq(r[1] / r[3], 0.0, 1e-4));
    }

    #[test]
    fn camera_view_proj_is_well_formed() {
        let cam = Camera2d::new([1920, 1080]);
        let vp = cam.view_proj();
        assert!(!mat_eq(&vp, &[[0.0; 4]; 4]));
        // Last row is [0,0,0,1] for an orthographic projection.
        assert!(approx_eq(vp[3][3], 1.0, 1e-4));
    }

    // Test helper: 4x4 matrix * column vec4. The matrix is stored column-major
    // (per `glam::Mat4::to_cols_array_2d`), so m[col][row].
    fn mul_mat4_vec4(m: &[[f32; 4]; 4], v: [f32; 4]) -> [f32; 4] {
        let mut result = [0.0_f32; 4];
        for (row, elem) in result.iter_mut().enumerate() {
            for col in 0..4 {
                *elem += m[col][row] * v[col];
            }
        }
        result
    }
}
