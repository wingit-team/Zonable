//! Draw call batching.

use canopy_assets::handle::{AssetId, Handle};
use canopy_assets::types::Mesh;
use canopy_ecs::entity::Entity;
use glam::Mat4;

/// A single draw call submitted to the renderer.
#[derive(Debug, Clone)]
pub struct DrawCall {
    pub entity: Entity,
    pub mesh_id: AssetId,
    pub material_id: AssetId,
    pub transform: Mat4,
    pub lod_level: u8,
    pub cast_shadow: bool,
    pub receive_shadow: bool,
}
