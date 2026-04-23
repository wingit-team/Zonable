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
use crate::frame::FrameTimer;
use crate::plugin::Plugin;
use crate::stage::AppStage;
use canopy_assets::AssetServer;
use canopy_ecs::{
    system::{BoxedSystem, FnSystem, System, SystemScheduler, SystemStage},
    world::World,
};
use canopy_platform::{
    event::CanopyEvent,
    window::{PlatformWindow, WindowConfig},
};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

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

        let mut frame_timer = FrameTimer::new(self.config.target_tick_hz, self.config.heartbeat_hz);
        let mut running = true;

        // winit 0.29 event loop
        let mut world = self.world;
        let mut scheduler = self.scheduler;

        event_loop.run(move |event, target| {
            use winit::event_loop::ControlFlow;
            target.set_control_flow(ControlFlow::Poll);

            if let Some(flow) = platform.handle_winit_event(event) {
                if flow == ControlFlow::Exit {
                    info!("Shutdown requested");
                    target.exit();
                    return;
                }
            }

            // Process accumulated events (filled by handle_winit_event)
            let events = platform.poll_events();
            for ev in &events {
                if matches!(ev, CanopyEvent::WindowCloseRequested) {
                    info!("Window close requested — shutting down");
                    target.exit();
                    return;
                }
            }

            // Frame tick
            let frame = frame_timer.tick();
            let dt = frame.dt;

            // Run all systems
            scheduler.run_all(&mut world, dt);

        }).expect("Event loop error");

        info!("Canopy Engine shut down cleanly");
    }

    // -----------------------------------------------------------------------
    // Internal
    // -----------------------------------------------------------------------

    fn init_logging(&self) {
        let level = self.config.log_level.as_tracing_level();
        let filter = EnvFilter::from_default_env()
            .add_directive(format!("canopy={}", level).parse().unwrap());
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .compact()
            .init();
    }
}
