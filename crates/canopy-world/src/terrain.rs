//! Terrain generation — noise-based heightmaps.

use noise::{NoiseFn, Perlin, Fbm, MultiFractal};
use glam::Vec2;

/// Generates terrain height values using layered Perlin noise (fBm).
pub struct TerrainGenerator {
    pub seed: u32,
    fbm: Fbm<Perlin>,
    pub height_scale: f32,
    pub horizontal_scale: f32,
}

impl TerrainGenerator {
    pub fn new(seed: u32, height_scale: f32, horizontal_scale: f32) -> Self {
        let mut fbm = Fbm::<Perlin>::new(seed);
        fbm.octaves = 8;
        fbm.frequency = 1.0;
        fbm.lacunarity = 2.0;
        fbm.persistence = 0.5;
        Self { seed, fbm, height_scale, horizontal_scale }
    }

    /// Sample height at world-space (x, z).
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        let nx = x as f64 / self.horizontal_scale as f64;
        let nz = z as f64 / self.horizontal_scale as f64;
        let n = self.fbm.get([nx, nz]) as f32;
        // Remap from [-1, 1] to [0, height_scale]
        (n + 1.0) * 0.5 * self.height_scale
    }

    /// Generate a heightmap for a chunk of `size × size` samples.
    pub fn generate_chunk(&self, origin_x: f32, origin_z: f32, chunk_size: f32, resolution: u32) -> Vec<f32> {
        let step = chunk_size / resolution as f32;
        let n = (resolution + 1) as usize;
        let mut heights = Vec::with_capacity(n * n);
        for row in 0..=resolution {
            for col in 0..=resolution {
                let x = origin_x + col as f32 * step;
                let z = origin_z + row as f32 * step;
                heights.push(self.height_at(x, z));
            }
        }
        heights
    }
}
