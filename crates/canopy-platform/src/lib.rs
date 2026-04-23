//! `canopy-platform` — Platform abstraction layer.
//!
//! Wraps `winit` to provide a stable, engine-internal API for:
//! - Window creation and management
//! - Input state snapshots (keyboard, mouse, gamepad)
//! - Unified event stream
//!
//! The rest of the engine depends on `canopy-platform` types, not `winit` types
//! directly. This ensures that swapping to a different windowing library in future
//! only requires changes here.

pub mod event;
pub mod input;
pub mod window;

pub use event::CanopyEvent;
pub use input::InputState;
pub use window::PlatformWindow;

pub mod prelude {
    pub use super::event::CanopyEvent;
    pub use super::input::{InputState, KeyCode, MouseButton};
    pub use super::window::{PlatformWindow, WindowConfig};
}
