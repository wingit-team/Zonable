//! Unified event enum — all platform events translated from winit.
//!
//! `CanopyEvent` is the only event type the rest of the engine consumes.
//! It is deliberately narrow: we only expose what game systems need.
//! Raw winit events are filtered and translated in `PlatformWindow::poll_events`.

use crate::input::{KeyCode, MouseButton};
use glam::Vec2;

/// A platform event from the OS or windowing system.
#[derive(Debug, Clone, PartialEq)]
pub enum CanopyEvent {
    // ------------------------------------------------------------------
    // Window events
    // ------------------------------------------------------------------
    /// Window was resized. Values are the new logical pixel dimensions.
    WindowResized { width: u32, height: u32 },
    /// Window close was requested (X button, Cmd+Q, etc.).
    WindowCloseRequested,
    /// Window gained or lost OS focus.
    WindowFocusChanged { focused: bool },
    /// Window was moved to a new position on screen.
    WindowMoved { x: i32, y: i32 },

    // ------------------------------------------------------------------
    // Keyboard events
    // ------------------------------------------------------------------
    KeyPressed { key: KeyCode, modifiers: Modifiers },
    KeyReleased { key: KeyCode, modifiers: Modifiers },
    /// A Unicode character was received (for text input, not game controls).
    CharReceived(char),

    // ------------------------------------------------------------------
    // Mouse events
    // ------------------------------------------------------------------
    MouseMoved { position: Vec2 },
    /// Raw delta since last frame (for camera, not affected by acceleration).
    MouseDelta { delta: Vec2 },
    MouseButtonPressed { button: MouseButton },
    MouseButtonReleased { button: MouseButton },
    /// Scroll wheel. `y` is vertical (main scroll), `x` is horizontal.
    MouseScrolled { delta: Vec2 },
    /// Mouse cursor entered/exited the window.
    MouseEntered,
    MouseLeft,

    // ------------------------------------------------------------------
    // Gamepad events (placeholder, full implementation Phase 2)
    // ------------------------------------------------------------------
    GamepadConnected { id: u32 },
    GamepadDisconnected { id: u32 },
    GamepadAxis { id: u32, axis: u8, value: f32 },
    GamepadButton { id: u32, button: u8, pressed: bool },
}

/// Keyboard modifier key state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_key: bool,
}
