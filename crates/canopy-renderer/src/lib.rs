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

pub mod context;
pub mod gpu_assets;
pub mod pipeline;
pub mod system;

pub use camera::Camera;
pub use draw::DrawCall;
pub use lod::LodSelector;
pub use context::RenderContext;
pub use gpu_assets::{GpuResourceManager, GpuMesh, GpuTexture};
pub use pipeline::StandardPipeline;

pub mod prelude {
    pub use super::camera::Camera;
    pub use super::draw::DrawCall;
    pub use super::lod::LodSelector;
}

use canopy_core::plugin::Plugin;
use canopy_core::app::CanopyApp;
use canopy_core::stage::AppStage;

/// Core renderer plugin.
/// Note: Since `RenderContext` requires a `PlatformWindow` which is created
/// *after* `CanopyApp` builds plugins, the initialization of wgpu happens
/// dynamically or must be manually orchestrated. For Phase 1, we provide the types.
pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut CanopyApp) {
        // Register the render system
        app.add_fn_system(AppStage::Render, "render_system", system::render_system);
    }
}
