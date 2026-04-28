//! `PlatformWindow` — winit window lifecycle and event translation.
//!
//! # Architecture
//!
//! `PlatformWindow` wraps a `winit::window::Window`. The engine main loop calls
//! `poll_events()` at the top of each frame, which:
//! 1. Drains the winit event queue
//! 2. Translates raw winit events → `CanopyEvent`
//! 3. Updates the `InputState` snapshot
//! 4. Returns the event list for systems to consume
//!
//! # Window creation
//!
//! `PlatformWindow::create(config)` must be called on the main thread (winit
//! requires this on macOS and some other platforms). The `EventLoop` is owned by
//! the window and must be run on the same thread.
//!
//! # Headless mode
//!
//! For server-side simulation (Zonable dedicated sim server) we support headless
//! mode where no actual window is created. The `PlatformWindow` still exists
//! but `poll_events` returns an empty vec and the renderer is disabled.

use crate::event::{CanopyEvent, Modifiers};
use crate::input::{InputState, KeyCode, MouseButton};
use glam::Vec2;
use tracing::info;
use winit::{
    event::{DeviceEvent, ElementState, Event, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{CursorGrabMode, Window, WindowBuilder},
};

/// Configuration for window creation.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub fullscreen: bool,
    pub vsync: bool,
    pub headless: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Canopy Engine".to_string(),
            width: 1920,
            height: 1080,
            resizable: true,
            fullscreen: false,
            vsync: true,
            headless: false,
        }
    }
}

/// Owns the OS window and event loop.
///
/// # Usage in the engine main loop
///
/// ```rust,no_run
/// use canopy_platform::window::{PlatformWindow, WindowConfig};
/// use canopy_platform::event::CanopyEvent;
///
/// let config = WindowConfig { title: "Zonable".into(), width: 1920, height: 1080, ..Default::default() };
/// let (mut window, event_loop) = PlatformWindow::create(config);
///
/// // The event loop is run via `canopy-core`'s EngineRunner which calls
/// // event_loop.run(...) and pumps `window.poll_events()` each iteration.
/// ```
pub struct PlatformWindow {
    pub config: WindowConfig,
    /// `None` in headless mode.
    pub inner: Option<std::sync::Arc<Window>>,
    pub input: InputState,
    /// Accumulated events since last `poll_events` call.
    pending_events: Vec<CanopyEvent>,
    pub logical_size: (u32, u32),
    pub physical_size: (u32, u32),
    last_mouse_position: Option<Vec2>,
    /// Whether the cursor is currently captured (grabbed + hidden).
    pub cursor_grabbed: bool,
}

impl PlatformWindow {
    /// Create a window and its event loop.
    ///
    /// Returns `(PlatformWindow, EventLoop<()>)`. The event loop is returned
    /// separately because `EventLoop::run` takes ownership and must be called
    /// from the main thread.
    ///
    /// In headless mode, the window is `None` but the struct is still valid.
    pub fn create(config: WindowConfig) -> (Self, EventLoop<()>) {
        let event_loop = EventLoopBuilder::new().build().expect("EventLoop creation failed");

        let inner = if config.headless {
            info!("Platform: headless mode — no window created");
            None
        } else {
            let window = WindowBuilder::new()
                .with_title(&config.title)
                .with_inner_size(winit::dpi::LogicalSize::new(config.width, config.height))
                .with_resizable(config.resizable)
                .build(&event_loop)
                .expect("Window creation failed");
            info!(
                "Platform: window created '{}' {}×{}",
                config.title, config.width, config.height
            );
            Some(std::sync::Arc::new(window))
        };

        let physical_size = inner.as_ref().map(|w| (w.inner_size().width, w.inner_size().height)).unwrap_or((config.width, config.height));

        let mut platform = Self {
            logical_size: (config.width, config.height),
            physical_size,
            config,
            inner,
            input: InputState::new(),
            pending_events: Vec::new(),
            last_mouse_position: None,
            cursor_grabbed: false,
        };

        // Do not grab cursor on startup — the user controls capture via
        // mouse click (grab) and Escape (release).
        // platform.set_cursor_grabbed(true);  // uncomment to start grabbed

        (platform, event_loop)
    }

    /// Drain and translate a raw winit `Event` into our `CanopyEvent` vocabulary.
    ///
    /// This is called inside `EventLoop::run`'s closure. The results are pushed
    /// into `pending_events` which `poll_events` drains.
    pub fn handle_winit_event(&mut self, event: Event<()>) -> bool {
        match event {
            Event::WindowEvent { event, .. } => {
                self.translate_window_event(event)
            }
            Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                let d = Vec2::new(delta.0 as f32, delta.1 as f32);
                self.input.accumulate_mouse_delta(d);
                self.pending_events.push(CanopyEvent::MouseDelta { delta: d });
                false
            }
            Event::AboutToWait => {
                false
            }
            _ => false,
        }
    }

    fn translate_window_event(&mut self, event: WindowEvent) -> bool {
        match event {
            WindowEvent::CloseRequested => {
                self.pending_events.push(CanopyEvent::WindowCloseRequested);
                true
            }
            WindowEvent::Resized(size) => {
                self.logical_size = (size.width, size.height); // Note: winit 0.29 Resized gives physical
                self.physical_size = (size.width, size.height);
                self.pending_events.push(CanopyEvent::WindowResized {
                    width: size.width,
                    height: size.height,
                });
                false
            }
            WindowEvent::Focused(focused) => {
                if focused && self.cursor_grabbed {
                    // Re-apply grab on focus regain only if we were grabbed before.
                    self.set_cursor_grabbed(true);
                } else if !focused {
                    // Always release the OS grab when we lose focus so the
                    // user can interact with other windows.
                    if let Some(window) = self.inner.as_ref() {
                        let _ = window.set_cursor_grab(CursorGrabMode::None);
                        window.set_cursor_visible(true);
                    }
                }
                self.pending_events.push(CanopyEvent::WindowFocusChanged { focused });
                false
            }
            WindowEvent::KeyboardInput { event, .. } => {
                // winit 0.29 uses `event.physical_key` and `event.logical_key`
                // We translate to our KeyCode best-effort; unmapped keys → Unknown
                let key = translate_key(&event);
                let modifiers = Modifiers::default(); // TODO: track modifier state
                match event.state {
                    ElementState::Pressed => {
                        // Escape releases cursor capture.
                        if key == KeyCode::Escape && self.cursor_grabbed {
                            self.set_cursor_grabbed(false);
                        }
                        self.input.set_key(key, true);
                        self.pending_events.push(CanopyEvent::KeyPressed { key, modifiers });
                    }
                    ElementState::Released => {
                        self.input.set_key(key, false);
                        self.pending_events.push(CanopyEvent::KeyReleased { key, modifiers });
                    }
                }
                false
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pos = Vec2::new(position.x as f32, position.y as f32);
                // Only track position here — do NOT accumulate delta from
                // CursorMoved. Raw deltas come from DeviceEvent::MouseMotion
                // which is already handled. Double-accumulating causes choppiness.
                self.last_mouse_position = Some(pos);
                self.input.set_mouse_position(pos);
                self.pending_events.push(CanopyEvent::MouseMoved { position: pos });
                false
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let btn = translate_mouse_button(button);
                match state {
                    ElementState::Pressed => {
                        self.input.set_mouse_button(btn, true);
                        // Re-capture cursor on any mouse click if not already grabbed.
                        if !self.cursor_grabbed {
                            self.set_cursor_grabbed(true);
                        }
                        self.pending_events.push(CanopyEvent::MouseButtonPressed { button: btn });
                    }
                    ElementState::Released => {
                        self.input.set_mouse_button(btn, false);
                        self.pending_events.push(CanopyEvent::MouseButtonReleased { button: btn });
                    }
                }
                false
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y) * 20.0,
                    MouseScrollDelta::PixelDelta(pos) => Vec2::new(pos.x as f32, pos.y as f32),
                };
                self.input.accumulate_scroll(scroll);
                self.pending_events.push(CanopyEvent::MouseScrolled { delta: scroll });
                false
            }
            _ => false,
        }
    }

    /// Drain the accumulated events for this frame. Call once per frame.
    pub fn poll_events(&mut self) -> Vec<CanopyEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Advance per-frame input snapshots after systems have run.
    pub fn end_frame(&mut self) {
        self.input.advance_frame();
    }

    /// Raw window handle for wgpu surface creation.
    pub fn raw_window_handle(&self) -> Option<std::sync::Arc<Window>> {
        self.inner.clone()
    }

    /// Grab or release the OS cursor.
    ///
    /// When grabbed: cursor is locked/confined and hidden (good for camera look).
    /// When released: cursor is visible and free (good for UI interaction).
    pub fn set_cursor_grabbed(&mut self, grabbed: bool) {
        self.cursor_grabbed = grabbed;
        if let Some(window) = self.inner.as_ref() {
            if grabbed {
                let _ = window.set_cursor_grab(CursorGrabMode::Locked)
                    .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
                window.set_cursor_visible(false);
            } else {
                let _ = window.set_cursor_grab(CursorGrabMode::None);
                window.set_cursor_visible(true);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Translation helpers
// ---------------------------------------------------------------------------

fn translate_key(event: &winit::event::KeyEvent) -> KeyCode {
    use winit::keyboard::{Key, NamedKey};
    match &event.logical_key {
        Key::Named(named) => match named {
            NamedKey::Space => KeyCode::Space,
            NamedKey::Enter => KeyCode::Enter,
            NamedKey::Escape => KeyCode::Escape,
            NamedKey::Backspace => KeyCode::Backspace,
            NamedKey::Tab => KeyCode::Tab,
            NamedKey::Delete => KeyCode::Delete,
            NamedKey::Insert => KeyCode::Insert,
            NamedKey::ArrowLeft => KeyCode::Left,
            NamedKey::ArrowRight => KeyCode::Right,
            NamedKey::ArrowUp => KeyCode::Up,
            NamedKey::ArrowDown => KeyCode::Down,
            NamedKey::Shift => KeyCode::LeftShift,
            NamedKey::Control => KeyCode::LeftCtrl,
            NamedKey::Alt => KeyCode::LeftAlt,
            NamedKey::Super => KeyCode::LeftSuper,
            NamedKey::F1 => KeyCode::F1,
            NamedKey::F2 => KeyCode::F2,
            NamedKey::F3 => KeyCode::F3,
            NamedKey::F4 => KeyCode::F4,
            NamedKey::F5 => KeyCode::F5,
            NamedKey::F6 => KeyCode::F6,
            NamedKey::F7 => KeyCode::F7,
            NamedKey::F8 => KeyCode::F8,
            NamedKey::F9 => KeyCode::F9,
            NamedKey::F10 => KeyCode::F10,
            NamedKey::F11 => KeyCode::F11,
            NamedKey::F12 => KeyCode::F12,
            _ => KeyCode::Unknown,
        },
        Key::Character(s) => match s.as_str() {
            "a" | "A" => KeyCode::A, "b" | "B" => KeyCode::B, "c" | "C" => KeyCode::C,
            "d" | "D" => KeyCode::D, "e" | "E" => KeyCode::E, "f" | "F" => KeyCode::F,
            "g" | "G" => KeyCode::G, "h" | "H" => KeyCode::H, "i" | "I" => KeyCode::I,
            "j" | "J" => KeyCode::J, "k" | "K" => KeyCode::K, "l" | "L" => KeyCode::L,
            "m" | "M" => KeyCode::M, "n" | "N" => KeyCode::N, "o" | "O" => KeyCode::O,
            "p" | "P" => KeyCode::P, "q" | "Q" => KeyCode::Q, "r" | "R" => KeyCode::R,
            "s" | "S" => KeyCode::S, "t" | "T" => KeyCode::T, "u" | "U" => KeyCode::U,
            "v" | "V" => KeyCode::V, "w" | "W" => KeyCode::W, "x" | "X" => KeyCode::X,
            "y" | "Y" => KeyCode::Y, "z" | "Z" => KeyCode::Z,
            "0" => KeyCode::Key0, "1" => KeyCode::Key1, "2" => KeyCode::Key2,
            "3" => KeyCode::Key3, "4" => KeyCode::Key4, "5" => KeyCode::Key5,
            "6" => KeyCode::Key6, "7" => KeyCode::Key7, "8" => KeyCode::Key8,
            "9" => KeyCode::Key9,
            _ => KeyCode::Unknown,
        },
        _ => KeyCode::Unknown,
    }
}

fn translate_mouse_button(button: winit::event::MouseButton) -> MouseButton {
    match button {
        winit::event::MouseButton::Left => MouseButton::Left,
        winit::event::MouseButton::Right => MouseButton::Right,
        winit::event::MouseButton::Middle => MouseButton::Middle,
        winit::event::MouseButton::Other(n) => MouseButton::Other(n as u8),
        _ => MouseButton::Other(255),
    }
}
