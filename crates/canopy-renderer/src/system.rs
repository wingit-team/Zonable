use crate::context::RenderContext;
use crate::debug::{classify_asset, ActiveOverlayPane, PerfToolkitState};
use crate::environment::RenderEnvironment;
use crate::gpu_assets::GpuResourceManager;
use crate::overlay::OverlayRenderer;
use crate::pipeline::StandardPipeline;
use crate::components::{Transform, MeshRef};
use canopy_ecs::world::World;
use canopy_assets::AssetServer;
use std::collections::BTreeMap;
use wgpu::util::DeviceExt;
use wgpu::{
    Color, LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor,
};
use tracing::{debug, error, warn};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [f32; 16],
    view: [f32; 16],
    proj: [f32; 16],
    position: [f32; 4],
    inv_view_proj: [f32; 16],
    sun_direction: [f32; 4],
    fog_color: [f32; 4],
    fog_params: [f32; 4],
    sky_top_color: [f32; 4],
    sky_horizon_color: [f32; 4],
    cel_params: [f32; 4],
    main_position: [f32; 4],
    main_forward: [f32; 4],
    main_right: [f32; 4],
    main_up: [f32; 4],
    main_fov_aspect: [f32; 4],
    pass_flags: [f32; 4],
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

fn build_camera_bind_group(
    device: &wgpu::Device,
    pipeline: &StandardPipeline,
    render_camera: &crate::camera::Camera,
    main_camera: &crate::camera::Camera,
    environment: &RenderEnvironment,
    secondary_pass: bool,
) -> wgpu::BindGroup {
    let view_mat = render_camera.view_matrix();
    let proj_mat = render_camera.projection_matrix();
    let main_right = main_camera.forward.cross(main_camera.up).normalize_or_zero();
    let main_up = main_right.cross(main_camera.forward).normalize_or_zero();
    let tan_half_y = (main_camera.fov_y_radians * 0.5).tan();
    let tan_half_x = tan_half_y * main_camera.aspect.max(0.0001);
    let uniform = CameraUniform {
        view_proj: (proj_mat * view_mat).to_cols_array(),
        view: view_mat.to_cols_array(),
        proj: proj_mat.to_cols_array(),
        position: [
            render_camera.position.x,
            render_camera.position.y,
            render_camera.position.z,
            1.0,
        ],
        inv_view_proj: (proj_mat * view_mat).inverse().to_cols_array(),
        sun_direction: [
            environment.sun_direction.x,
            environment.sun_direction.y,
            environment.sun_direction.z,
            0.0,
        ],
        fog_color: [
            environment.fog_color[0],
            environment.fog_color[1],
            environment.fog_color[2],
            1.0,
        ],
        fog_params: [environment.fog_density.max(0.0), environment.fog_start.max(0.0), 0.0, 0.0],
        sky_top_color: [
            environment.sky_top_color[0],
            environment.sky_top_color[1],
            environment.sky_top_color[2],
            1.0,
        ],
        sky_horizon_color: [
            environment.sky_horizon_color[0],
            environment.sky_horizon_color[1],
            environment.sky_horizon_color[2],
            1.0,
        ],
        cel_params: [environment.cel_shading_steps.max(1.0), 0.0, 0.0, 0.0],
        main_position: [
            main_camera.position.x,
            main_camera.position.y,
            main_camera.position.z,
            1.0,
        ],
        main_forward: [main_camera.forward.x, main_camera.forward.y, main_camera.forward.z, 0.0],
        main_right: [main_right.x, main_right.y, main_right.z, 0.0],
        main_up: [main_up.x, main_up.y, main_up.z, 0.0],
        main_fov_aspect: [tan_half_y, tan_half_x, main_camera.near, main_camera.far],
        pass_flags: [if secondary_pass { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
    };

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
}

fn is_visible_to_main_camera(camera: &crate::camera::Camera, transform: &Transform) -> bool {
    let to_obj = transform.position - camera.position;
    let distance_along_forward = to_obj.dot(camera.forward);
    let radius = transform.scale.max_element().max(1.0) * 0.866;
    if distance_along_forward < camera.near - radius || distance_along_forward > camera.far + radius {
        return false;
    }

    let right = camera.forward.cross(camera.up).normalize_or_zero();
    if right.length_squared() <= f32::EPSILON {
        return true;
    }
    let up = right.cross(camera.forward).normalize_or_zero();
    let x = to_obj.dot(right).abs();
    let y = to_obj.dot(up).abs();
    let half_y = (camera.fov_y_radians * 0.5).tan() * distance_along_forward.max(0.0);
    let half_x = half_y * camera.aspect.max(0.0001);

    x <= half_x + radius && y <= half_y + radius
}

pub fn render_system(world: &mut World, _dt: f64) {
    // 1. Immutable phase: Collect renderable entities
    let main_camera = world
        .get_resource::<crate::camera::Camera>()
        .cloned()
        .unwrap_or_else(|| crate::camera::Camera::new(45.0, 1.0));
    let entities = world.query_filtered(&[
        canopy_ecs::component::ComponentId::of::<Transform>(),
        canopy_ecs::component::ComponentId::of::<MeshRef>(),
    ]);

    let mut render_list = Vec::new();
    let mut class_counts: BTreeMap<String, usize> = BTreeMap::new();
    for entity in entities {
        if let (Some(t), Some(m)) = (world.get::<Transform>(entity), world.get::<MeshRef>(entity)) {
            if !is_visible_to_main_camera(&main_camera, t) {
                continue;
            }
            render_list.push((*t, m.clone()));
            let class_name = classify_asset(&m.asset);
            *class_counts.entry(class_name).or_insert(0) += 1;
        }
    }

    let total_entities = world.entity_count();
    if let Some(toolkit) = world.get_resource_mut::<PerfToolkitState>() {
        toolkit.entity_count = total_entities;
        let mut classes: Vec<(String, usize)> = class_counts.into_iter().collect();
        classes.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        toolkit.visible_classes = classes;
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
    let main_camera = if let Some(camera) = world.get_resource_mut::<crate::camera::Camera>() {
        camera.aspect = surface_config
            .as_ref()
            .map(|c| c.width as f32 / c.height as f32)
            .unwrap_or(1.0);
        camera.clone()
    } else {
        return;
    };
    debug!("render_system: camera view_proj={:?}", main_camera.view_projection());

    let environment = world
        .get_resource::<RenderEnvironment>()
        .cloned()
        .unwrap_or_default();

    let secondary_camera = world
        .get_resource::<PerfToolkitState>()
        .and_then(|toolkit| {
            if toolkit.enabled && toolkit.active_overlay == Some(ActiveOverlayPane::SecondaryCamera) {
                Some(toolkit.secondary_camera.camera.clone())
            } else {
                None
            }
        });
    let toolkit_snapshot = world.get_resource::<PerfToolkitState>().cloned();

    let camera_bind_group = build_camera_bind_group(
        &device,
        &pipeline,
        &main_camera,
        &main_camera,
        &environment,
        false,
    );
    let secondary_camera_bind_group = secondary_camera
        .as_ref()
        .map(|camera| {
            build_camera_bind_group(
                &device,
                &pipeline,
                camera,
                &main_camera,
                &environment,
                true,
            )
        });

    // 4. Encode Render Pass
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Main Command Encoder"),
    });

    {
        let depth_view = {
            let context = world.get_resource::<RenderContext>().unwrap();
            context.depth_view.as_ref().map(|v| v)
        };

        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Forward Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
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

        rpass.set_pipeline(&pipeline.sky_pipeline);
        rpass.set_bind_group(0, &camera_bind_group, &[]);
        rpass.draw(0..3, 0..1);

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
        let gpu_resources = world.get_resource_mut::<GpuResourceManager>().unwrap();
        gpu_resources.begin_frame();

        let mut draw_list = |rpass: &mut wgpu::RenderPass<'_>| {
            for (transform, mesh_ref) in &render_list {
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
                        debug!(
                            "render_system: drawing {} (indices={}) at pos={:?}, rot={:?}",
                            mesh_ref.asset,
                            gpu_mesh.index_count,
                            transform.position,
                            transform.rotation
                        );

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
                        gpu_resources.mark_mesh_used(handle.id);
                    }
                } else {
                    warn!("render_system: mesh {} has no LoD data", mesh_ref.asset);
                }
            }
        };

        draw_list(&mut rpass);

        // Secondary debug camera (F3+W): draw in a separate render pass to avoid transparency blending
        if let Some(secondary_bg) = secondary_camera_bind_group.as_ref() {
            let (surface_w, surface_h) = surface_config
                .as_ref()
                .map(|c| (c.width as f32, c.height as f32))
                .unwrap_or((1280.0, 720.0));
            let panel_w = surface_w * 0.34;
            let panel_h = surface_h * 0.34;
            let panel_x = surface_w - panel_w - 16.0;
            let panel_y = surface_h - panel_h - 16.0;

            drop(rpass); // Explicitly end the render pass
            
            let mut secondary_rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Secondary Camera Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            secondary_rpass.set_viewport(panel_x, panel_y, panel_w, panel_h, 0.0, 1.0);
            secondary_rpass.set_scissor_rect(panel_x as u32, panel_y as u32, panel_w as u32, panel_h as u32);
            secondary_rpass.set_pipeline(&pipeline.forward_pipeline);
            secondary_rpass.set_bind_group(0, secondary_bg, &[]);

            // Re-create material bind group for secondary pass
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
                label: Some("Secondary Material Bind Group"),
                layout: &pipeline.material_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&white_view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
                    wgpu::BindGroupEntry { binding: 2, resource: mat_buffer.as_entire_binding() },
                ],
            });
            secondary_rpass.set_bind_group(1, &material_bind_group, &[]);

            for (transform, mesh_ref) in &render_list {
                let handle = asset_server.register(&mesh_ref.asset);

                if let Err(e) = asset_server.load_sync(&mesh_ref.asset) {
                    error!("render_system: failed to load {}: {:?}", mesh_ref.asset, e);
                    continue;
                }

                if let Some(lods) = asset_server.get_lod_set(&handle) {
                    if let Some(gpu_mesh) = gpu_resources.get_mesh(handle.id) {
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

                        secondary_rpass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                        secondary_rpass.set_vertex_buffer(1, instance_buffer.slice(..));

                        let index_format = if gpu_mesh.index_u32 {
                            wgpu::IndexFormat::Uint32
                        } else {
                            wgpu::IndexFormat::Uint16
                        };

                        secondary_rpass.set_index_buffer(gpu_mesh.index_buffer.slice(..), index_format);
                        secondary_rpass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
                        gpu_resources.mark_mesh_used(handle.id);
                    }
                }
            }
        }
    }

    if let (Some(toolkit), Some(overlay)) = (
        toolkit_snapshot.as_ref(),
        world.get_resource_mut::<OverlayRenderer>(),
    ) {
        let (surface_w, surface_h) = surface_config
            .as_ref()
            .map(|c| (c.width, c.height))
            .unwrap_or((1280, 720));
        overlay.render(
            &device,
            &mut encoder,
            &view,
            surface_w,
            surface_h,
            toolkit,
        );
    }

    queue.submit(std::iter::once(encoder.finish()));
    surface_texture.present();
}
