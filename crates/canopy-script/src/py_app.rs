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
    ) -> Self {
        Self {
            title, width, height, vsync, headless,
            assets_dir, scripts_dir, target_tick_hz, heartbeat_hz, asset_memory_mb,
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
        // Release the GIL before entering the engine main loop — the engine
        // will re-acquire it when it needs to call Python systems.
        Python::with_gil(|py| {
            py.allow_threads(|| {
                CanopyApp::new(engine_config).run();
            });
            Ok(())
        })
    }

    fn __repr__(&self) -> String {
        format!("CanopyApp({})", self.config.__repr__())
    }
}
