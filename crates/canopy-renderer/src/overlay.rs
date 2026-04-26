use crate::debug::{ActiveOverlayPane, PerfToolkitState};
use font8x8::{BASIC_FONTS, UnicodeFonts};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct OverlayVertex {
    pos: [f32; 2],
    color: [f32; 4],
}

pub struct OverlayRenderer {
    pipeline: wgpu::RenderPipeline,
}

impl OverlayRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Overlay Shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VsIn) -> VsOut {
    var out: VsOut;
    out.pos = vec4<f32>(input.pos, 0.0, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    return input.color;
}
"#
                .into(),
            ),
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Overlay Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Overlay Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<OverlayVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self { pipeline }
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        width: u32,
        height: u32,
        toolkit: &PerfToolkitState,
    ) {
        if !toolkit.enabled {
            return;
        }

        let mut vertices = Vec::with_capacity(32_768);
        let line_height = 20.0;

        // Top-left compact HUD panel.
        self.push_rect_px(
            &mut vertices,
            width,
            height,
            8.0,
            8.0,
            600.0,
            80.0,
            [0.02, 0.04, 0.06, 0.80],
        );

        let top1 = format!(
            "FPS {:.1}   1% LOW {:.1}   LAT {:.2}ms",
            toolkit.fps_average, toolkit.fps_1pct_low, toolkit.latency_ms
        );
        self.push_text_px(&mut vertices, width, height, 16.0, 20.0, &top1, [0.8, 1.0, 0.8, 1.0]);

        if toolkit.active_overlay == Some(ActiveOverlayPane::EntityBreakdown) {
            let top2 = format!("ENTITIES {}", toolkit.entity_count);
            self.push_text_px(
                &mut vertices,
                width,
                height,
                16.0,
                20.0 + line_height,
                &top2,
                [0.95, 0.95, 0.6, 1.0],
            );
        }

        // Bottom-left pane.
        self.push_rect_px(
            &mut vertices,
            width,
            height,
            8.0,
            height as f32 - 220.0,
            600.0,
            210.0,
            [0.02, 0.04, 0.06, 0.78],
        );

        let bottom_pane_y = height as f32 - 210.0;

        match toolkit.active_overlay {
            Some(ActiveOverlayPane::FpsGraph) => {
                self.push_text_px(
                    &mut vertices,
                    width,
                    height,
                    16.0,
                    bottom_pane_y,
                    "FPS GRAPH",
                    [0.7, 0.9, 1.0, 1.0],
                );
                self.push_fps_graph(&mut vertices, width, height, toolkit, 20.0, bottom_pane_y + line_height);
            }
            Some(ActiveOverlayPane::SecondaryCamera) => {
                self.push_text_px(
                    &mut vertices,
                    width,
                    height,
                    16.0,
                    bottom_pane_y,
                    "SECONDARY CAMERA",
                    [0.7, 0.9, 1.0, 1.0],
                );
                self.push_text_px(
                    &mut vertices,
                    width,
                    height,
                    16.0,
                    bottom_pane_y + line_height,
                    "ARROWS ORBIT  Q/E ZOOM  WASD PAN",
                    [0.9, 0.9, 0.9, 1.0],
                );
            }
            Some(ActiveOverlayPane::EntityBreakdown) => {
                self.push_text_px(
                    &mut vertices,
                    width,
                    height,
                    16.0,
                    bottom_pane_y,
                    "VISIBLE RENDER CLASSES",
                    [0.7, 0.9, 1.0, 1.0],
                );
                for (i, (name, count)) in toolkit.visible_classes.iter().take(8).enumerate() {
                    let line = format!("{}: {}", name.to_ascii_uppercase(), count);
                    self.push_text_px(
                        &mut vertices,
                        width,
                        height,
                        16.0,
                        bottom_pane_y + line_height + (i as f32 * line_height),
                        &line,
                        [0.9, 0.9, 0.9, 1.0],
                    );
                }
            }
            Some(ActiveOverlayPane::SystemStats) => {
                self.push_text_px(
                    &mut vertices,
                    width,
                    height,
                    16.0,
                    bottom_pane_y,
                    "SYSTEM STATS",
                    [0.7, 0.9, 1.0, 1.0],
                );
                let cpu = format!("CPU {:.1}%  {}", toolkit.system_stats.cpu_usage_percent, toolkit.system_stats.cpu_name);
                let ram = format!("RAM {} / {} MB", toolkit.system_stats.ram_used_mb, toolkit.system_stats.ram_total_mb);
                let gpu = format!("GPU {}", toolkit.system_stats.gpu_name);
                self.push_text_px(&mut vertices, width, height, 16.0, bottom_pane_y + line_height, &cpu, [0.9, 0.9, 0.9, 1.0]);
                self.push_text_px(&mut vertices, width, height, 16.0, bottom_pane_y + line_height * 2.0, &ram, [0.9, 0.9, 0.9, 1.0]);
                self.push_text_px(&mut vertices, width, height, 16.0, bottom_pane_y + line_height * 3.0, &gpu, [0.9, 0.9, 0.9, 1.0]);
            }
            Some(ActiveOverlayPane::Help) => {
                self.push_text_px(
                    &mut vertices,
                    width,
                    height,
                    16.0,
                    bottom_pane_y,
                    "F3 HELP",
                    [0.7, 0.9, 1.0, 1.0],
                );
                self.push_text_px(
                    &mut vertices,
                    width,
                    height,
                    16.0,
                    bottom_pane_y + line_height,
                    "G GRAPH  W CAMERA  E ENTITIES  S SYSTEM",
                    [0.9, 0.9, 0.9, 1.0],
                );
                self.push_text_px(
                    &mut vertices,
                    width,
                    height,
                    16.0,
                    bottom_pane_y + line_height * 2.0,
                    "H HELP  C CULLING  L TIMINGS",
                    [0.9, 0.9, 0.9, 1.0],
                );
            }
            Some(ActiveOverlayPane::Culling) => {
                let vis_count: usize = toolkit.visible_classes.iter().map(|(_, c)| *c).sum();
                let line = format!("VISIBLE {}  CLASS TYPES {}", vis_count, toolkit.visible_classes.len());
                self.push_text_px(
                    &mut vertices,
                    width,
                    height,
                    16.0,
                    bottom_pane_y,
                    "CULLING",
                    [0.7, 0.9, 1.0, 1.0],
                );
                self.push_text_px(
                    &mut vertices,
                    width,
                    height,
                    16.0,
                    bottom_pane_y + line_height,
                    &line,
                    [0.9, 0.9, 0.9, 1.0],
                );
            }
            Some(ActiveOverlayPane::Timings) => {
                let line = format!("FRAME {:.2}ms  TARGET {:.2}ms", toolkit.latency_ms, 1000.0 / 60.0);
                self.push_text_px(
                    &mut vertices,
                    width,
                    height,
                    16.0,
                    bottom_pane_y,
                    "TIMINGS",
                    [0.7, 0.9, 1.0, 1.0],
                );
                self.push_text_px(
                    &mut vertices,
                    width,
                    height,
                    16.0,
                    bottom_pane_y + line_height,
                    &line,
                    [0.9, 0.9, 0.9, 1.0],
                );
            }
            None => {
                self.push_text_px(
                    &mut vertices,
                    width,
                    height,
                    16.0,
                    bottom_pane_y,
                    "F3 ACTIVE",
                    [0.7, 0.9, 1.0, 1.0],
                );
                self.push_text_px(
                    &mut vertices,
                    width,
                    height,
                    16.0,
                    bottom_pane_y + line_height,
                    "PRESS F3+H FOR HELP",
                    [0.9, 0.9, 0.9, 1.0],
                );
            }
        }

        if vertices.is_empty() {
            return;
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Overlay Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Overlay Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
        rpass.draw(0..vertices.len() as u32, 0..1);
    }

    fn push_fps_graph(
        &self,
        out: &mut Vec<OverlayVertex>,
        width: u32,
        height: u32,
        toolkit: &PerfToolkitState,
        x: f32,
        y: f32,
    ) {
        let graph_w = 480.0;
        let graph_h = 150.0;
        self.push_rect_px(out, width, height, x, y, graph_w, graph_h, [0.05, 0.08, 0.1, 0.8]);

        if toolkit.fps_history.len() < 2 {
            return;
        }

        let max_fps = toolkit
            .fps_history
            .iter()
            .copied()
            .fold(1.0_f32, f32::max)
            .max(30.0);

        let samples: Vec<f32> = toolkit.fps_history.iter().copied().collect();
        let step = graph_w / samples.len().max(1) as f32;
        for (i, fps) in samples.iter().enumerate() {
            let norm = (fps / max_fps).clamp(0.0, 1.0);
            let bar_h = norm * (graph_h - 6.0);
            self.push_rect_px(
                out,
                width,
                height,
                x + 3.0 + i as f32 * step,
                y + graph_h - 3.0 - bar_h,
                step.max(1.0),
                bar_h,
                [0.35, 0.9, 0.5, 0.95],
            );
        }
    }

    fn push_text_px(
        &self,
        out: &mut Vec<OverlayVertex>,
        width: u32,
        height: u32,
        x: f32,
        y: f32,
        text: &str,
        color: [f32; 4],
    ) {
        let scale = 2.0f32;
        let mut pen_x = x;
        let mut pen_y = y;

        for ch in text.chars() {
            if ch == '\n' {
                pen_x = x;
                pen_y += 10.0 * scale;
                continue;
            }
            if let Some(glyph) = BASIC_FONTS.get(ch) {
                for (row, bits) in glyph.iter().enumerate() {
                    for col in 0..8 {
                        if (*bits >> col) & 1 != 0 {
                            self.push_rect_px(
                                out,
                                width,
                                height,
                                pen_x + col as f32 * scale,
                                pen_y + row as f32 * scale,
                                scale,
                                scale,
                                color,
                            );
                        }
                    }
                }
            }
            pen_x += 8.0 * scale;
        }
    }

    fn push_rect_px(
        &self,
        out: &mut Vec<OverlayVertex>,
        width: u32,
        height: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [f32; 4],
    ) {
        let x0 = (x / width as f32) * 2.0 - 1.0;
        let y0 = 1.0 - (y / height as f32) * 2.0;
        let x1 = ((x + w) / width as f32) * 2.0 - 1.0;
        let y1 = 1.0 - ((y + h) / height as f32) * 2.0;

        out.push(OverlayVertex { pos: [x0, y0], color });
        out.push(OverlayVertex { pos: [x1, y0], color });
        out.push(OverlayVertex { pos: [x1, y1], color });

        out.push(OverlayVertex { pos: [x0, y0], color });
        out.push(OverlayVertex { pos: [x1, y1], color });
        out.push(OverlayVertex { pos: [x0, y1], color });
    }
}
