//! Rendering system — extracts ECS state and submits wgpu draw calls.

use crate::context::RenderContext;
use crate::gpu_assets::{GpuResourceManager, MAX_GPU_MEMORY_BYTES};
use crate::Camera;
use crate::pipeline::StandardPipeline;
use canopy_ecs::world::World;
use std::time::Instant;
use tracing::{debug, error, trace};
use wgpu::{Color, LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor};

#[derive(Clone)]
pub struct MockTransform {
    pub position: glam::Vec3,
}

#[derive(Clone)]
pub struct MockMeshRef {
    pub asset: String,
}

/// Main render system running in `AppStage::Render`.
pub fn render_system(world: &mut World, _dt: f64) {
    let start = Instant::now();

    // 1. Acquire Resources
    // Need a mutable borrow of GpuResourceManager to mark usage/upload
    if let Some(gpu_resources) = world.get_resource_mut::<GpuResourceManager>() {
        gpu_resources.begin_frame();
    }

    let context = match world.get_resource::<RenderContext>() {
        Some(ctx) => ctx,
        None => return, // No renderer available
    };

    let pipeline = match world.get_resource::<StandardPipeline>() {
        Some(p) => p,
        None => return,
    };

    // 2. Extract Camera (Fallback if none found)
    let camera_entities = world.query_filtered(&[
        canopy_ecs::component::ComponentId::of::<Camera>(),
        canopy_ecs::component::ComponentId::of::<MockTransform>(),
    ]);

    let camera_data = if let Some(&entity) = camera_entities.first() {
        let cam = world.get::<Camera>(entity).unwrap();
        let transform = world.get::<MockTransform>(entity).unwrap();
        (cam.clone(), transform.clone())
    } else {
        // Fallback camera
        let cam = Camera::new(60.0, context.surface_config.as_ref().map(|c| c.width as f32 / c.height as f32).unwrap_or(1.0));
        let transform = MockTransform { position: glam::Vec3::new(0.0, 0.0, 100.0) };
        (cam, transform)
    };

    let (camera, cam_transform) = camera_data;

    // 3. Extract Renderables & evaluate LoD
    let renderable_entities = world.query_filtered(&[
        canopy_ecs::component::ComponentId::of::<MockTransform>(),
        canopy_ecs::component::ComponentId::of::<MockMeshRef>(),
    ]);

    let mut draw_calls: Vec<crate::DrawCall> = Vec::new();
    
    // In Phase 1, we just mock the AssetId for the asset string, since AssetServer integration
    // requires pulling actual loaded `.canasset` data which we'll refine in Phase 2.
    // For now, we just pretend we have the assets if they were mocked.

    // 4. Acquire Swapchain Texture
    let surface_texture = match context.get_current_texture() {
        Ok(tex) => tex,
        Err(wgpu::SurfaceError::Outdated) => {
            // Reconfigure surface on next frame
            return;
        }
        Err(e) => {
            error!("Failed to acquire next surface texture: {:?}", e);
            return;
        }
    };

    let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

    // 5. Encode Render Pass
    let mut encoder = context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Main Command Encoder"),
    });

    {
        // Simple Forward Pass to clear the screen
        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Forward Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None, // No depth for now in this mocked pass
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        rpass.set_pipeline(&pipeline.forward_pipeline);
        // We would set bind groups and draw here
        // rpass.draw(0..3, 0..1); // Draw a triangle if we wanted
    }

    // 6. Submit
    context.queue.submit(std::iter::once(encoder.finish()));
    surface_texture.present();

    let duration = start.elapsed();
    trace!("Render pass completed in {:?}", duration);
}
