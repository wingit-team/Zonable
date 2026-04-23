//! `canopy-renderer` — wgpu-based rendering pipeline.
//!
//! # Phase 1 Status: Scaffold
//!
//! This crate contains data structures and API surface but no working GPU code.
//! Phase 2 will implement:
//! - wgpu Device/Queue/Surface initialization
//! - PBR material system with bindless textures
//! - Multi-pass render pipeline (depth pre-pass, GBuffer, lighting, post-process)
//! - LoD selection via `LodSelector` (screen coverage + distance)
//! - GPU-side occlusion culling (occlusion queries + hierarchical Z-buffer)
//! - Instanced draw calls for repeated objects (trees, buildings)
//! - Shadow mapping (cascaded shadow maps for sun, point/spot for city lights)

pub mod camera;
pub mod draw;
pub mod lod;

pub use camera::Camera;
pub use draw::DrawCall;
pub use lod::LodSelector;

pub mod prelude {
    pub use super::camera::Camera;
    pub use super::draw::DrawCall;
    pub use super::lod::LodSelector;
}
