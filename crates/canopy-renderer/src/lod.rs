//! LoD selection.

use glam::Vec3;

/// Selects the appropriate LoD level for a mesh given camera position.
pub struct LodSelector {
    /// Screen-space thresholds for each LoD level.
    /// lod_thresholds[i] = min screen coverage ratio to use lod i.
    pub coverage_thresholds: [f32; 4],
}

impl Default for LodSelector {
    fn default() -> Self {
        Self {
            coverage_thresholds: [0.20, 0.05, 0.01, 0.0],
        }
    }
}

impl LodSelector {
    /// Select LoD level (0=highest quality) given screen coverage [0.0, 1.0].
    pub fn select(&self, coverage: f32) -> u8 {
        for (i, &threshold) in self.coverage_thresholds.iter().enumerate() {
            if coverage >= threshold {
                return i as u8;
            }
        }
        3
    }

    /// Estimate screen coverage given object radius and distance from camera.
    pub fn estimate_coverage(
        radius: f32,
        distance: f32,
        fov_y_radians: f32,
        viewport_height: u32,
    ) -> f32 {
        if distance <= 0.0 {
            return 1.0;
        }
        // Projected radius in pixels (approximation)
        let projected_radius =
            (radius / distance) * (viewport_height as f32 / (2.0 * (fov_y_radians / 2.0).tan()));
        let screen_diameter = 2.0 * projected_radius;
        (screen_diameter / viewport_height as f32).min(1.0)
    }
}
