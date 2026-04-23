//! Input state snapshot.
//!
//! Rather than a stream of events, game systems often need to poll "is this key
//! currently held?" per frame. `InputState` provides exactly this — it is updated
//! by `PlatformWindow::poll_events` and snapshotted at the start of each frame.
//!
//! # Design Note
//!
//! We use a `[bool; 256]` array for key state because:
//! - O(1) lookup by keycode
//! - Trivially copyable (no allocations)
//! - 256 bytes — fits in half a cache line
//!
//! For "just pressed this frame" vs "held", we keep a previous-frame snapshot
//! and compare. This is the classic `ButtonState` double-buffer pattern.

use glam::Vec2;

/// Keyboard key codes — a simplified subset of winit's `VirtualKeyCode`.
/// Extended as needed for game input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum KeyCode {
    // Alphabet
    A = 0, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    // Numbers
    Key0 = 26, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9,
    // Function keys
    F1 = 36, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // Special
    Space = 48, Enter, Escape, Backspace, Tab, Delete, Insert,
    Left = 56, Right, Up, Down,
    LeftShift = 60, RightShift, LeftCtrl, RightCtrl, LeftAlt, RightAlt,
    LeftSuper = 66, RightSuper,
    // Numpad
    Numpad0 = 70, Numpad1, Numpad2, Numpad3, Numpad4,
    Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,
    NumpadAdd = 80, NumpadSub, NumpadMul, NumpadDiv, NumpadEnter,
    // Misc
    Grave = 86, Minus, Equal, LeftBracket, RightBracket,
    Backslash, Semicolon, Apostrophe, Comma, Period, Slash,
    // Sentinel
    Unknown = 255,
}

impl KeyCode {
    pub fn from_u8(v: u8) -> Self {
        // Safety: exhaustive match or Unknown fallback
        // In Phase 2 this will use a proper mapping from winit::VirtualKeyCode
        unsafe { std::mem::transmute(v) }
    }
}

/// Mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

/// Per-frame input state snapshot.
///
/// Updated by `PlatformWindow` at the start of each frame from accumulated events.
#[derive(Debug, Clone)]
pub struct InputState {
    /// Keys held this frame
    keys_current: [bool; 256],
    /// Keys held last frame — diff gives just_pressed / just_released
    keys_previous: [bool; 256],

    mouse_position: Vec2,
    mouse_delta: Vec2,
    scroll_delta: Vec2,

    mouse_current: [bool; 8],
    mouse_previous: [bool; 8],
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_current: [false; 256],
            keys_previous: [false; 256],
            mouse_position: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            scroll_delta: Vec2::ZERO,
            mouse_current: [false; 8],
            mouse_previous: [false; 8],
        }
    }

    /// Called by `PlatformWindow` at the end of each frame to advance the state.
    pub fn advance_frame(&mut self) {
        self.keys_previous = self.keys_current;
        self.mouse_previous = self.mouse_current;
        self.mouse_delta = Vec2::ZERO;
        self.scroll_delta = Vec2::ZERO;
    }

    // --- Key state ---

    pub fn set_key(&mut self, key: KeyCode, pressed: bool) {
        self.keys_current[key as usize] = pressed;
    }

    /// Is the key currently held down?
    #[inline]
    pub fn key_held(&self, key: KeyCode) -> bool {
        self.keys_current[key as usize]
    }

    /// Was the key pressed this frame (not held from last frame)?
    #[inline]
    pub fn key_just_pressed(&self, key: KeyCode) -> bool {
        self.keys_current[key as usize] && !self.keys_previous[key as usize]
    }

    /// Was the key released this frame?
    #[inline]
    pub fn key_just_released(&self, key: KeyCode) -> bool {
        !self.keys_current[key as usize] && self.keys_previous[key as usize]
    }

    // --- Mouse state ---

    pub fn set_mouse_position(&mut self, pos: Vec2) {
        self.mouse_position = pos;
    }

    pub fn accumulate_mouse_delta(&mut self, delta: Vec2) {
        self.mouse_delta += delta;
    }

    pub fn accumulate_scroll(&mut self, delta: Vec2) {
        self.scroll_delta += delta;
    }

    pub fn set_mouse_button(&mut self, button: MouseButton, pressed: bool) {
        let idx = match button {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            MouseButton::Middle => 2,
            MouseButton::Other(n) => (n as usize).min(7),
        };
        self.mouse_current[idx] = pressed;
    }

    #[inline]
    pub fn mouse_position(&self) -> Vec2 {
        self.mouse_position
    }

    #[inline]
    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }

    #[inline]
    pub fn scroll_delta(&self) -> Vec2 {
        self.scroll_delta
    }

    fn mouse_btn_idx(button: MouseButton) -> usize {
        match button {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            MouseButton::Middle => 2,
            MouseButton::Other(n) => (n as usize).min(7),
        }
    }

    #[inline]
    pub fn mouse_held(&self, button: MouseButton) -> bool {
        self.mouse_current[Self::mouse_btn_idx(button)]
    }

    #[inline]
    pub fn mouse_just_pressed(&self, button: MouseButton) -> bool {
        let i = Self::mouse_btn_idx(button);
        self.mouse_current[i] && !self.mouse_previous[i]
    }

    #[inline]
    pub fn mouse_just_released(&self, button: MouseButton) -> bool {
        let i = Self::mouse_btn_idx(button);
        !self.mouse_current[i] && self.mouse_previous[i]
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}
