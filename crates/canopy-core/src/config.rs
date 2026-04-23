//! Engine configuration — parsed from file or set programmatically.

use std::path::PathBuf;

/// Top-level engine configuration.
///
/// Passed to `CanopyApp::new(config)`. In Python this is constructed via
/// `canopy.EngineConfig(title="Zonable", ...)`.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    // --- Window ---
    pub title: String,
    pub resolution: (u32, u32),
    pub fullscreen: bool,
    pub vsync: bool,
    pub headless: bool,

    // --- Paths ---
    /// Root directory for game assets. Passed to `AssetServer`.
    pub assets_dir: PathBuf,
    /// Root directory for Python game scripts.
    pub scripts_dir: PathBuf,

    // --- Simulation ---
    /// Target game loop tick rate (Hz). Default 60.
    pub target_tick_hz: u32,
    /// Heartbeat simulation rate for out-of-view entities (Hz). Default 4.
    pub heartbeat_hz: u32,
    /// Radius (metres) within which entities run at full tick rate.
    pub active_sim_radius: f32,

    // --- Renderer ---
    /// Maximum asset memory budget (MB).
    pub asset_memory_mb: usize,
    /// MSAA samples (1, 2, 4, 8).
    pub msaa_samples: u8,
    /// Shadow map resolution.
    pub shadow_map_size: u32,

    // --- Developer ---
    pub enable_profiler: bool,
    pub log_level: LogLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn as_tracing_level(&self) -> tracing::Level {
        match self {
            Self::Error => tracing::Level::ERROR,
            Self::Warn => tracing::Level::WARN,
            Self::Info => tracing::Level::INFO,
            Self::Debug => tracing::Level::DEBUG,
            Self::Trace => tracing::Level::TRACE,
        }
    }
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            title: "Canopy Engine".to_string(),
            resolution: (1920, 1080),
            fullscreen: false,
            vsync: true,
            headless: false,
            assets_dir: PathBuf::from("assets"),
            scripts_dir: PathBuf::from("scripts"),
            target_tick_hz: 60,
            heartbeat_hz: 4,
            active_sim_radius: 500.0,
            asset_memory_mb: 2048,
            msaa_samples: 4,
            shadow_map_size: 4096,
            enable_profiler: false,
            log_level: LogLevel::Info,
        }
    }
}
