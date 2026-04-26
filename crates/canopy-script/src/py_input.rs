//! Python Input API — exposes keyboard and mouse state to scripting.

use canopy_platform::InputState;
use canopy_platform::input::KeyCode;
use pyo3::prelude::*;
use std::cell::RefCell;

thread_local! {
    static CURRENT_INPUT: RefCell<Option<InputState>> = RefCell::new(None);
}

/// Set the active input state for this thread. Called by ScriptRunner.
pub fn set_active_input(input: InputState) {
    CURRENT_INPUT.with(|i| {
        *i.borrow_mut() = Some(input);
    });
}

/// Clear the active input state.
pub fn clear_active_input() {
    CURRENT_INPUT.with(|i| {
        *i.borrow_mut() = None;
    });
}

fn with_input<R>(f: impl FnOnce(&InputState) -> R) -> R {
    CURRENT_INPUT.with(|cell| {
        let input = cell.borrow();
        let input = input.as_ref().expect("Input state not set — are you calling input APIs outside of a system/tick?");
        f(input)
    })
}

#[pyclass(name = "Input")]
pub struct PyInput;

#[pymethods]
impl PyInput {
    /// Is the key currently held down?
    ///
    /// ```python
    /// if input.is_key_held("A"):
    ///     # do something
    /// ```
    #[staticmethod]
    pub fn is_key_held(key_name: &str) -> bool {
        let key = parse_key_name(key_name);
        with_input(|i| i.key_held(key))
    }

    /// Was the key pressed this frame?
    #[staticmethod]
    pub fn is_key_just_pressed(key_name: &str) -> bool {
        let key = parse_key_name(key_name);
        with_input(|i| i.key_just_pressed(key))
    }

    /// Was the key released this frame?
    #[staticmethod]
    pub fn is_key_just_released(key_name: &str) -> bool {
        let key = parse_key_name(key_name);
        with_input(|i| i.key_just_released(key))
    }
}

fn parse_key_name(name: &str) -> KeyCode {
    match name.to_uppercase().as_str() {
        "A" => KeyCode::A, "B" => KeyCode::B, "C" => KeyCode::C, "D" => KeyCode::D,
        "E" => KeyCode::E, "F" => KeyCode::F, "G" => KeyCode::G, "H" => KeyCode::H,
        "I" => KeyCode::I, "J" => KeyCode::J, "K" => KeyCode::K, "L" => KeyCode::L,
        "M" => KeyCode::M, "N" => KeyCode::N, "O" => KeyCode::O, "P" => KeyCode::P,
        "Q" => KeyCode::Q, "R" => KeyCode::R, "S" => KeyCode::S, "T" => KeyCode::T,
        "U" => KeyCode::U, "V" => KeyCode::V, "W" => KeyCode::W, "X" => KeyCode::X,
        "Y" => KeyCode::Y, "Z" => KeyCode::Z,
        "SPACE" => KeyCode::Space, "ENTER" => KeyCode::Enter, "ESCAPE" => KeyCode::Escape,
        "LEFT" => KeyCode::Left, "RIGHT" => KeyCode::Right, "UP" => KeyCode::Up, "DOWN" => KeyCode::Down,
        _ => KeyCode::Unknown,
    }
}
