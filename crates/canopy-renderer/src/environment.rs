#[derive(Debug, Clone)]
pub struct RenderEnvironment {
    pub cel_shading_steps: f32,
    pub sun_direction: glam::Vec3,
    pub fog_density: f32,
    pub fog_start: f32,
    pub fog_color: [f32; 3],
    pub sky_top_color: [f32; 3],
    pub sky_horizon_color: [f32; 3],
}

impl Default for RenderEnvironment {
    fn default() -> Self {
        Self {
            cel_shading_steps: 4.0,
            // Sun points down and towards scene origin from a clear angle.
            sun_direction: glam::Vec3::new(-0.45, -0.85, -0.30).normalize(),
            fog_density: 0.028,
            fog_start: 8.0,
            fog_color: [0.68, 0.77, 0.90],
            sky_top_color: [0.22, 0.45, 0.80],
            sky_horizon_color: [0.68, 0.77, 0.90],
        }
    }
}

