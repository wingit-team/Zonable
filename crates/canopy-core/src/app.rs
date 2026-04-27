//! `CanopyApp` — the central engine builder and runner.
//!
//! # Lifecycle
//!
//! ```text
//! CanopyApp::new(config)
//!   .add_plugin(...)
//!   .add_system(...)
//!   .run()          ← blocks until shutdown
//! ```
//!
//! Inside `run()`:
//! 1. Initialize logging (tracing)
//! 2. Build the platform window + event loop
//! 3. Initialize `AssetServer` with configured budget
//! 4. Call `plugin.build(self)` for each registered plugin (in order)
//! 5. Enter the winit event loop:
//!    a. `platform.poll_events()` — translate OS events
//!    b. Check for `WindowCloseRequested` → clean shutdown
//!    c. `frame_timer.tick()` → compute FrameData
//!    d. `scheduler.run_all(world, dt)` — run systems
//!    e. Repeat
//! 6. On shutdown: flush command buffers, drop all resources

use crate::config::EngineConfig;
use crate::camera_controller::{orbit_camera_system, OrbitCameraState};
use crate::frame::FrameTimer;
use crate::plugin::Plugin;
use crate::stage::AppStage;
use canopy_assets::AssetServer;
use canopy_ecs::{
    system::{FnSystem, System, SystemScheduler, SystemStage},
    world::World,
};
use canopy_platform::{
    event::CanopyEvent,
    window::{PlatformWindow, WindowConfig},
};
use canopy_renderer::{
    GpuResourceManager, OverlayRenderer, PerfToolkitState, RenderContext, StandardPipeline,
    RenderEnvironment,
    system::render_system,
};
use tracing::info;
use tracing_subscriber::EnvFilter;
use sysinfo::System as SysinfoSystem;

pub struct CanopyApp {
    pub config: EngineConfig,
    pub world: World,
    pub asset_server: AssetServer,
    scheduler: SystemScheduler,
    plugins: Vec<Box<dyn Plugin>>,
}

impl CanopyApp {
    /// Create a new app with the given config.
    pub fn new(config: EngineConfig) -> Self {
        Self {
            asset_server: AssetServer::new(
                config.assets_dir.clone(),
                config.asset_memory_mb,
            ),
            config,
            world: World::new(),
            scheduler: SystemScheduler::new(),
            plugins: Vec::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(EngineConfig::default())
    }

    // -----------------------------------------------------------------------
    // Builder API
    // -----------------------------------------------------------------------

    /// Register a plugin. Returns `self` for chaining.
    pub fn add_plugin<P: Plugin>(mut self, plugin: P) -> Self {
        self.plugins.push(Box::new(plugin));
        self
    }

    /// Add a system in the `Update` stage.
    pub fn add_system<S: System>(&mut self, system: S) {
        self.scheduler.add_system(system);
    }

    /// Add a system to a specific stage.
    pub fn add_system_to_stage<S: System>(&mut self, stage: AppStage, system: S) {
        let ecs_stage: SystemStage = stage.into();
        self.scheduler.add_system_to_stage(ecs_stage, system);
    }

    /// Add a system from a plain function.
    pub fn add_fn_system(
        &mut self,
        stage: AppStage,
        name: &'static str,
        func: impl Fn(&mut World, f64) + Send + Sync + 'static,
    ) {
        let system = FnSystem::new(func)
            .with_stage(stage.into())
            .with_name(name);
        self.scheduler.add_system(system);
    }

    // -----------------------------------------------------------------------
    // Run
    // -----------------------------------------------------------------------

    /// Build plugins and enter the engine main loop.
    ///
    /// This function takes ownership of `self` and blocks until the window is
    /// closed or a `Shutdown` event is raised. Never returns in normal operation.
    pub fn run(mut self) {
        self.init_logging();
        info!("Canopy Engine starting — '{}'", self.config.title);

        // Build all registered plugins
        let plugins = std::mem::take(&mut self.plugins);
        for plugin in &plugins {
            info!("Building plugin: {}", plugin.name());
            // Safety: we need &mut self while iterating plugins.
            // This is safe because we took ownership above.
            plugin.build(&mut self);
        }

        let window_config = WindowConfig {
            title: self.config.title.clone(),
            width: self.config.resolution.0,
            height: self.config.resolution.1,
            fullscreen: self.config.fullscreen,
            vsync: self.config.vsync,
            headless: self.config.headless,
            resizable: true,
        };

        let (mut platform, event_loop) = PlatformWindow::create(window_config);

        // Initialize Renderer
        let context = pollster::block_on(RenderContext::new(&platform));
        let gpu_name = context.adapter.get_info().name;
        let pipeline = StandardPipeline::new(&context.device, context.surface_format);
        let overlay_renderer = OverlayRenderer::new(&context.device, context.surface_format);
        let gpu_manager = GpuResourceManager::default();

        let mut perf_toolkit = PerfToolkitState::default();
        perf_toolkit.system_stats.gpu_name = gpu_name;
        let render_environment = RenderEnvironment {
            cel_shading_steps: self.config.cel_shading_steps,
            sun_direction: glam::Vec3::from_array(self.config.sun_direction).normalize_or_zero(),
            fog_density: self.config.fog_density,
            fog_start: self.config.fog_start,
            fog_color: self.config.sky_horizon_color,
            sky_top_color: self.config.sky_top_color,
            sky_horizon_color: self.config.sky_horizon_color,
        };

        self.world.insert_resource(context);
        self.world.insert_resource(pipeline);
        self.world.insert_resource(overlay_renderer);
        self.world.insert_resource(gpu_manager);
        self.world.insert_resource(perf_toolkit);
        self.world.insert_resource(render_environment);
        self.world.insert_resource(self.asset_server.clone());

        // Default Camera
        let mut camera = canopy_renderer::Camera::new(45.0, 1.0);
        camera.position = glam::Vec3::new(3.0, 3.0, 3.0);
        camera.forward = (glam::Vec3::ZERO - camera.position).normalize();
        self.world.insert_resource(camera);
        self.world.insert_resource(OrbitCameraState::default());

        self.add_fn_system(AppStage::PreUpdate, "orbit_camera_system", orbit_camera_system);
        
        // Add render system if not already present
        self.add_fn_system(AppStage::Render, "render_system", render_system);

        let mut frame_timer = FrameTimer::new(self.config.target_tick_hz, self.config.heartbeat_hz);

        // winit 0.29 event loop
        let mut world = self.world;
        let mut scheduler = self.scheduler;
        let mut system_info = SysinfoSystem::new_all();
        let mut device_events_enabled = false;
        let cpu_name = system_info
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());

        event_loop.run(move |event, target| {
            use winit::event_loop::ControlFlow;
            target.set_control_flow(ControlFlow::Poll);
            if !device_events_enabled {
                target.listen_device_events(winit::event_loop::DeviceEvents::Always);
                device_events_enabled = true;
            }

            if platform.handle_winit_event(event) {
                info!("Shutdown requested");
                target.exit();
                return;
            }

            // Process accumulated events (filled by handle_winit_event)
            let events = platform.poll_events();
            for ev in &events {
                match ev {
                    CanopyEvent::WindowCloseRequested => {
                        info!("Window close requested — shutting down");
                        target.exit();
                        return;
                    }
                    CanopyEvent::WindowResized { width, height } => {
                        let mut context = world.get_resource_mut::<RenderContext>().unwrap();
                        context.resize(*width, *height);
                    }
                    _ => {}
                }
            }

            // Frame tick
            let frame = frame_timer.tick();
            let dt = frame.dt;

            // Update input resource in world
            world.insert_resource(platform.input.clone());

            // Keep F3 toolkit state engine-global and available to all games.
            let main_camera_snapshot = world.get_resource::<canopy_renderer::Camera>().cloned();
            if let Some(toolkit) = world.get_resource_mut::<PerfToolkitState>() {
                toolkit.update_toggle_state(&platform.input);
                toolkit.update_frame_metrics(dt);
                toolkit.update_secondary_camera(dt, main_camera_snapshot.as_ref());
                if toolkit.enabled {
                    system_info.refresh_cpu_usage();
                    system_info.refresh_memory();
                    toolkit.system_stats.cpu_name = cpu_name.clone();
                    let cpu_len = system_info.cpus().len().max(1) as f32;
                    toolkit.system_stats.cpu_usage_percent =
                        system_info.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpu_len;
                    toolkit.system_stats.ram_total_mb = system_info.total_memory() / (1024 * 1024);
                    toolkit.system_stats.ram_used_mb = system_info.used_memory() / (1024 * 1024);
                    toolkit.system_stats.gpu_usage_percent = None;
                }
            }

            // Run all systems
            scheduler.run_all(&mut world, dt);


            // Snapshot current input as previous-frame input only after systems run.
            platform.end_frame();

        }).expect("Event loop error");

        info!("Canopy Engine shut down cleanly");
    }

    // -----------------------------------------------------------------------
    // Internal
    // -----------------------------------------------------------------------

    fn init_logging(&self) {
        let level = self.config.log_level.as_tracing_level();
        let filter = EnvFilter::from_default_env()
            .add_directive(format!("canopy_core={}", level).parse().unwrap())
            .add_directive(format!("canopy_renderer={}", level).parse().unwrap())
            .add_directive(format!("canopy_platform={}", level).parse().unwrap())
            .add_directive(format!("canopy_script={}", level).parse().unwrap());
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .compact()
            .init();
    }
}
