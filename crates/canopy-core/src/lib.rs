//! `canopy-core` — Engine orchestration: app loop, plugin system, lifecycle.
//!
//! This is the top-level crate that glues everything together. Game binaries
//! (or the Python extension) create a `CanopyApp`, register plugins, and call
//! `CanopyApp::run()` which takes over the main thread.
//!
//! # Plugin System
//!
//! Inspired by Bevy's plugin model. Every engine subsystem is a `Plugin`:
//!
//! ```rust
//! use canopy_core::{CanopyApp, plugin::Plugin};
//!
//! struct MyGamePlugin;
//! impl Plugin for MyGamePlugin {
//!     fn build(&self, app: &mut CanopyApp) {
//!         app.add_system(canopy_ecs::system::SystemStage::Update, my_system);
//!     }
//! }
//!
//! CanopyApp::new()
//!     .add_plugin(MyGamePlugin)
//!     .run();
//! ```

pub mod app;
pub mod camera_controller;
pub mod config;
pub mod frame;
pub mod plugin;
pub mod stage;

pub use app::CanopyApp;
pub use camera_controller::OrbitCameraState;
pub use config::EngineConfig;
pub use frame::FrameData;
pub use plugin::Plugin;
pub use stage::AppStage;

pub mod prelude {
    pub use super::app::CanopyApp;
    pub use super::config::EngineConfig;
    pub use super::frame::FrameData;
    pub use super::plugin::Plugin;
    pub use super::stage::AppStage;
    pub use canopy_ecs::prelude::*;
}
