//! Chunk streaming and world map.

use ahash::AHashMap;
use canopy_ecs::entity::Entity;
use glam::Vec3;

/// 2D chunk coordinate in the world grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkCoord {
    pub x: i32,
    pub z: i32,
}

impl ChunkCoord {
    pub fn from_world_pos(pos: Vec3, chunk_size: f32) -> Self {
        Self {
            x: (pos.x / chunk_size).floor() as i32,
            z: (pos.z / chunk_size).floor() as i32,
        }
    }

    pub fn world_origin(&self, chunk_size: f32) -> Vec3 {
        Vec3::new(self.x as f32 * chunk_size, 0.0, self.z as f32 * chunk_size)
    }

    /// Manhattan distance between two chunk coords.
    pub fn chebyshev_distance(&self, other: ChunkCoord) -> u32 {
        ((self.x - other.x).abs().max((self.z - other.z).abs())) as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkState {
    Unloaded,
    Queued,
    Loading,
    Ready,
    Evicting,
}

/// A single streaming chunk. Owns entity references for all entities within.
pub struct Chunk {
    pub coord: ChunkCoord,
    pub state: ChunkState,
    pub entities: Vec<Entity>,
    /// Height samples for terrain collision queries (flattened 2D grid)
    pub heightmap: Vec<f32>,
    pub heightmap_resolution: u32,
}

/// Global world map — owns chunk slots.
pub struct WorldMap {
    pub chunks: AHashMap<ChunkCoord, Chunk>,
    pub chunk_size_meters: f32,
}

impl WorldMap {
    pub fn new(chunk_size: f32) -> Self {
        Self { chunks: AHashMap::new(), chunk_size_meters: chunk_size }
    }

    pub fn get_chunk(&self, coord: ChunkCoord) -> Option<&Chunk> {
        self.chunks.get(&coord)
    }

    pub fn coord_for_pos(&self, pos: Vec3) -> ChunkCoord {
        ChunkCoord::from_world_pos(pos, self.chunk_size_meters)
    }

    pub fn sample_height(&self, pos: Vec3) -> f32 {
        // TODO Phase 2: bilinear interpolation from heightmap
        0.0
    }
}

/// Predictive chunk loader.
///
/// # Phase 2 Implementation
///
/// Each frame, given camera position + velocity:
/// 1. Compute `look_ahead_pos = camera_pos + velocity * look_ahead_seconds`
/// 2. List all chunk coords within `stream_radius` of both current AND look_ahead pos
/// 3. Queue load for any unloaded chunks in that set (sorted by distance — near-first)
/// 4. Evict chunks beyond `evict_radius` (furthest first, respecting entity ref count)
///
/// The streamer uses a background Tokio task to do the actual disk I/O.
pub struct ChunkStreamer {
    pub stream_radius_chunks: u32,
    pub evict_radius_chunks: u32,
    pub look_ahead_seconds: f32,
}

impl ChunkStreamer {
    pub fn new() -> Self {
        Self {
            stream_radius_chunks: 5,
            evict_radius_chunks: 8,
            look_ahead_seconds: 2.0,
        }
    }

    /// Determine which chunks need to be loaded given current camera state.
    pub fn required_chunks(&self, camera_pos: Vec3, camera_vel: Vec3, chunk_size: f32) -> Vec<ChunkCoord> {
        let look_ahead = camera_pos + camera_vel * self.look_ahead_seconds;
        let current_coord = ChunkCoord::from_world_pos(camera_pos, chunk_size);
        let ahead_coord = ChunkCoord::from_world_pos(look_ahead, chunk_size);

        let r = self.stream_radius_chunks as i32;
        let mut coords = Vec::new();

        // Union of circles around current and look-ahead positions
        for x in (current_coord.x - r)..=(current_coord.x + r) {
            for z in (current_coord.z - r)..=(current_coord.z + r) {
                let coord = ChunkCoord { x, z };
                if coord.chebyshev_distance(current_coord) <= self.stream_radius_chunks
                    || coord.chebyshev_distance(ahead_coord) <= self.stream_radius_chunks
                {
                    coords.push(coord);
                }
            }
        }

        // Sort by distance from camera (load nearest first)
        coords.sort_by_key(|c| c.chebyshev_distance(current_coord));
        coords
    }
}

impl Default for ChunkStreamer {
    fn default() -> Self { Self::new() }
}
