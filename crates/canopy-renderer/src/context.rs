//! `RenderContext` — wgpu initialization and management.

use canopy_platform::window::PlatformWindow;
use tracing::{error, info};
use wgpu::{
    Adapter, Device, Instance, Queue, Surface, SurfaceConfiguration, SurfaceError, TextureFormat,
};

/// Manages the core `wgpu` state.
pub struct RenderContext {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: std::sync::Arc<Device>,
    pub queue: std::sync::Arc<Queue>,
    pub surface: Option<Surface<'static>>,
    pub surface_config: Option<SurfaceConfiguration>,
    pub surface_format: TextureFormat,
    pub depth_view: Option<wgpu::TextureView>,
}

impl RenderContext {
    /// Initialize wgpu asynchronously.
    pub async fn new(window: &PlatformWindow) -> Self {
        info!("Initializing wgpu...");

        // Create an instance targeting all available backends (Vulkan, Metal, DX12, Browser WebGPU)
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Create a surface if we have a window (not headless)
        let surface = if let Some(w) = window.raw_window_handle() {
            Some(
                instance
                    .create_surface(w)
                    .expect("Failed to create wgpu surface"),
            )
        } else {
            None
        };

        // Request an adapter (physical device)
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: surface.as_ref(),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        info!("Selected GPU: {}", adapter.get_info().name);

        // Request a device (logical device) and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Canopy Render Device"),
                    required_features: wgpu::Features::empty(), // Add features as needed in Phase 2
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None, // Trace path
            )
            .await
            .expect("Failed to create wgpu device");
            
        let device = std::sync::Arc::new(device);
        let queue = std::sync::Arc::new(queue);

        // Configure the surface if it exists
        let mut surface_config = None;
        let mut surface_format = TextureFormat::Bgra8UnormSrgb; // Default fallback

        if let Some(surface) = &surface {
            let surface_caps = surface.get_capabilities(&adapter);
            
            // Prefer an sRGB format
            surface_format = surface_caps
                .formats
                .iter()
                .copied()
                .find(|f| f.is_srgb())
                .unwrap_or(surface_caps.formats[0]);

            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width: window.physical_size.0,
                height: window.physical_size.1,
                present_mode: if window.config.vsync {
                    wgpu::PresentMode::Fifo
                } else {
                    wgpu::PresentMode::Immediate
                },
                alpha_mode: surface_caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };

            surface.configure(&device, &config);
            surface_config = Some(config);
            
            info!("Surface configured: {}x{} ({:?})", window.physical_size.0, window.physical_size.1, surface_format);
        }

        let depth_view = if !window.config.headless {
            Some(Self::create_depth_view(&device, window.physical_size.0, window.physical_size.1))
        } else {
            None
        };

        Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            surface_config,
            surface_format,
            depth_view,
        }
    }

    /// Resize the surface. Must be called when the window resizes.
    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width > 0 && new_height > 0 {
            if let Some(config) = &mut self.surface_config {
                config.width = new_width;
                config.height = new_height;
                if let Some(surface) = &self.surface {
                    surface.configure(&self.device, config);
                }
                self.depth_view = Some(Self::create_depth_view(&self.device, new_width, new_height));
            }
        }
    }

    fn create_depth_view(device: &Device, width: u32, height: u32) -> wgpu::TextureView {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Get the next frame from the surface.
    pub fn get_current_texture(&self) -> Result<wgpu::SurfaceTexture, SurfaceError> {
        if let Some(surface) = &self.surface {
            surface.get_current_texture()
        } else {
            Err(SurfaceError::Outdated)
        }
    }
}
