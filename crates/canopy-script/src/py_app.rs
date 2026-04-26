//! `CanopyApp` Python entry point.

use canopy_core::{CanopyApp, EngineConfig};
use pyo3::prelude::*;

#[pyclass(name = "EngineConfig")]
#[derive(Debug, Clone)]
pub struct PyEngineConfig {
    #[pyo3(get, set)]
    pub title: String,
    #[pyo3(get, set)]
    pub width: u32,
    #[pyo3(get, set)]
    pub height: u32,
    #[pyo3(get, set)]
    pub vsync: bool,
    #[pyo3(get, set)]
    pub headless: bool,
    #[pyo3(get, set)]
    pub assets_dir: String,
    #[pyo3(get, set)]
    pub scripts_dir: String,
    #[pyo3(get, set)]
    pub target_tick_hz: u32,
    #[pyo3(get, set)]
    pub heartbeat_hz: u32,
    #[pyo3(get, set)]
    pub asset_memory_mb: usize,
    #[pyo3(get, set)]
    pub cel_shading_steps: f32,
    #[pyo3(get, set)]
    pub sun_direction: (f32, f32, f32),
    #[pyo3(get, set)]
    pub fog_density: f32,
    #[pyo3(get, set)]
    pub fog_start: f32,
    #[pyo3(get, set)]
    pub sky_horizon_color: (f32, f32, f32),
    #[pyo3(get, set)]
    pub sky_top_color: (f32, f32, f32),
}

#[pymethods]
impl PyEngineConfig {
    #[new]
    #[pyo3(signature = (
        title = "Canopy Engine".to_string(),
        width = 1920,
        height = 1080,
        vsync = true,
        headless = false,
        assets_dir = "assets".to_string(),
        scripts_dir = "scripts".to_string(),
        target_tick_hz = 60,
        heartbeat_hz = 4,
        asset_memory_mb = 2048,
        cel_shading_steps = 4.0,
        sun_direction = (-0.45, -0.85, -0.30),
        fog_density = 0.028,
        fog_start = 8.0,
        sky_horizon_color = (0.68, 0.77, 0.90),
        sky_top_color = (0.22, 0.45, 0.80),
    ))]
    pub fn new(
        title: String,
        width: u32,
        height: u32,
        vsync: bool,
        headless: bool,
        assets_dir: String,
        scripts_dir: String,
        target_tick_hz: u32,
        heartbeat_hz: u32,
        asset_memory_mb: usize,
        cel_shading_steps: f32,
        sun_direction: (f32, f32, f32),
        fog_density: f32,
        fog_start: f32,
        sky_horizon_color: (f32, f32, f32),
        sky_top_color: (f32, f32, f32),
    ) -> Self {
        Self {
            title, width, height, vsync, headless,
            assets_dir, scripts_dir, target_tick_hz, heartbeat_hz, asset_memory_mb,
            cel_shading_steps,
            sun_direction,
            fog_density,
            fog_start,
            sky_horizon_color,
            sky_top_color,
        }
    }

    fn __repr__(&self) -> String {
        format!("EngineConfig(title='{}', {}x{}, vsync={})", self.title, self.width, self.height, self.vsync)
    }
}

impl PyEngineConfig {
    pub fn to_engine_config(&self) -> EngineConfig {
        EngineConfig {
            title: self.title.clone(),
            resolution: (self.width, self.height),
            vsync: self.vsync,
            headless: self.headless,
            assets_dir: self.assets_dir.clone().into(),
            scripts_dir: self.scripts_dir.clone().into(),
            target_tick_hz: self.target_tick_hz,
            heartbeat_hz: self.heartbeat_hz,
            asset_memory_mb: self.asset_memory_mb,
            cel_shading_steps: self.cel_shading_steps,
            sun_direction: [self.sun_direction.0, self.sun_direction.1, self.sun_direction.2],
            fog_density: self.fog_density,
            fog_start: self.fog_start,
            sky_horizon_color: [
                self.sky_horizon_color.0,
                self.sky_horizon_color.1,
                self.sky_horizon_color.2,
            ],
            sky_top_color: [self.sky_top_color.0, self.sky_top_color.1, self.sky_top_color.2],
            ..Default::default()
        }
    }
}

/// Python-accessible engine entry point.
///
/// ```python
/// from canopy import CanopyApp, EngineConfig
///
/// config = EngineConfig(title="Zonable", width=2560, height=1440)
/// app = CanopyApp(config)
/// app.run()  # Blocks until window closed
/// ```
#[pyclass(name = "CanopyApp")]
pub struct PyCanopyApp {
    config: PyEngineConfig,
}

#[pymethods]
impl PyCanopyApp {
    #[new]
    pub fn new(config: PyEngineConfig) -> Self {
        Self { config }
    }

    /// Start the engine. Blocks until shutdown.
    pub fn run(&self) -> PyResult<()> {
        let engine_config = self.config.to_engine_config();
        let scripts_dir = self.config.scripts_dir.clone();
        let target_tick_hz = self.config.target_tick_hz;

        Python::with_gil(|py| {
            // 1. Create and prepare the ScriptRunner
            let mut runner = crate::runner::ScriptRunner::new(scripts_dir, target_tick_hz);
            runner.load_scripts(py)?;

            // 2. Release GIL and run the engine
            py.allow_threads(|| {
                let mut app = canopy_core::app::CanopyApp::new(engine_config);
                
                // Add the ScriptPlugin which bridges Python logic to the engine
                app = app.add_plugin(crate::runner::ScriptPlugin::new(runner));
                
                app.run();
            });
            Ok(())
        })
    }

    fn __repr__(&self) -> String {
        format!("CanopyApp({})", self.config.__repr__())
    }
}
