use crate::context::RenderContext;
use crate::gpu_assets::GpuResourceManager;
use crate::pipeline::StandardPipeline;
use crate::components::{Transform, MeshRef};
use canopy_ecs::world::World;
use canopy_assets::AssetServer;
use wgpu::util::DeviceExt;
use wgpu::{
    Color, LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor,
};
use tracing::{error, info, warn};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [f32; 16],
    view: [f32; 16],
    proj: [f32; 16],
    position: [f32; 4],
    inv_view_proj: [f32; 16],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct MaterialUniforms {
    base_color: [f32; 4],
    roughness: f32,
    metallic: f32,
    padding1: f32,
    padding2: f32,
}

pub fn render_system(world: &mut World, _dt: f64) {
    // 1. Immutable phase: Collect renderable entities
    let entities = world.query_filtered(&[
        canopy_ecs::component::ComponentId::of::<Transform>(),
        canopy_ecs::component::ComponentId::of::<MeshRef>(),
    ]);

    let mut render_list = Vec::new();
    for entity in entities {
        if let (Some(t), Some(m)) = (world.get::<Transform>(entity), world.get::<MeshRef>(entity)) {
            render_list.push((*t, m.clone()));
        }
    }

    // 2. Resource phase: Extract context and prepare surface
    let (device, queue, pipeline, asset_server, view, surface_texture, surface_config) = {
        let context = world.get_resource::<RenderContext>().unwrap();
        let pipeline = world.get_resource::<StandardPipeline>().unwrap().clone();
        let asset_server = world.get_resource::<AssetServer>().unwrap().clone();
        
        let surface = context.surface.as_ref().expect("RenderContext missing surface");
        let surface_texture = match surface.get_current_texture() {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to acquire surface texture: {:?}", e);
                return;
            }
        };
        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        (
            context.device.clone(),
            context.queue.clone(),
            pipeline,
            asset_server,
            view,
            surface_texture,
            context.surface_config.clone(),
        )
    };
    
    // Now 'context' is dropped, we can borrow world mutably for camera
    let camera_bind_group = if let Some(mut camera) = world.get_resource_mut::<crate::camera::Camera>() {
        camera.aspect = surface_config.as_ref().map(|c| c.width as f32 / c.height as f32).unwrap_or(1.0);
        
        let view_mat = camera.view_matrix();
        let proj_mat = camera.projection_matrix();
        let uniform = CameraUniform {
            view_proj: (proj_mat * view_mat).to_cols_array(),
            view: view_mat.to_cols_array(),
            proj: proj_mat.to_cols_array(),
            position: [camera.position.x, camera.position.y, camera.position.z, 1.0],
            inv_view_proj: (proj_mat * view_mat).inverse().to_cols_array(),
        };
        
        info!("render_system: camera view_proj={:?}", proj_mat * view_mat);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &pipeline.camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        })
    } else {
        return;
    };

    // 4. Encode Render Pass
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Main Command Encoder"),
    });

    {
        let depth_view = {
            let context = world.get_resource::<RenderContext>().unwrap();
            context.depth_view.as_ref().map(|v| v.clone())
        };

        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Forward Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color { r: 0.5, g: 0.5, b: 0.8, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: depth_view.as_ref().map(|v| RenderPassDepthStencilAttachment {
                view: v,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        rpass.set_pipeline(&pipeline.forward_pipeline);
        rpass.set_bind_group(0, &camera_bind_group, &[]);

        // Dummy material setup
        let white_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("White Texture"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &white_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255, 255, 255, 255],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        );
        let white_view = white_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let mat_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Buffer"),
            contents: bytemuck::cast_slice(&[1.0f32, 1.0, 1.0, 1.0, 0.5, 0.0, 0.0, 0.0]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let material_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Default Material Bind Group"),
            layout: &pipeline.material_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&white_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: mat_buffer.as_entire_binding() },
            ],
        });
        rpass.set_bind_group(1, &material_bind_group, &[]);

        // Re-acquire GpuResourceManager
        let mut gpu_resources = world.get_resource_mut::<GpuResourceManager>().unwrap();

        for (transform, mesh_ref) in render_list {
            let handle = asset_server.register(&mesh_ref.asset);
            
            if let Err(e) = asset_server.load_sync(&mesh_ref.asset) {
                error!("render_system: failed to load {}: {:?}", mesh_ref.asset, e);
                continue;
            }

            if let Some(lods) = asset_server.get_lod_set(&handle) {
                let mesh = &lods.lods[0];
                if gpu_resources.get_mesh(handle.id).is_none() {
                    gpu_resources.upload_mesh(&device, handle.id, mesh);
                }

                if let Some(gpu_mesh) = gpu_resources.get_mesh(handle.id) {
                    info!("render_system: drawing {} (indices={}) at pos={:?}, rot={:?}", 
                        mesh_ref.asset, gpu_mesh.index_count, transform.position, transform.rotation);
                    
                    let model_matrix = glam::Mat4::from_scale_rotation_translation(
                        transform.scale,
                        transform.rotation,
                        transform.position,
                    );

                    let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Instance Buffer"),
                        contents: bytemuck::cast_slice(&model_matrix.to_cols_array()),
                        usage: wgpu::BufferUsages::VERTEX,
                    });

                    rpass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                    rpass.set_vertex_buffer(1, instance_buffer.slice(..));
                    
                    let index_format = if gpu_mesh.index_u32 {
                        wgpu::IndexFormat::Uint32
                    } else {
                        wgpu::IndexFormat::Uint16
                    };

                    rpass.set_index_buffer(gpu_mesh.index_buffer.slice(..), index_format);
                    rpass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
                }
            } else {
                warn!("render_system: mesh {} has no LoD data", mesh_ref.asset);
            }
        }
    }

    queue.submit(std::iter::once(encoder.finish()));
    surface_texture.present();
}
