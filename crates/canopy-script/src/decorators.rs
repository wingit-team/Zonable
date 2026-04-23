//! Python decorator functions — @on_event, @on_tick, @on_init.
//!
//! These are Python-callable functions that register handlers into a
//! module-level `_registry` dict. The `ScriptRunner` reads this registry
//! after importing all game scripts.
//!
//! # Usage in Python
//!
//! ```python
//! from canopy import on_event, on_tick, on_init
//! from canopy.sim import EarthquakeEvent
//!
//! @on_event(EarthquakeEvent)
//! def handle_quake(event):
//!     print(f"Earthquake at {event.epicenter}!")
//!
//! @on_tick(rate_hz=4)
//! def budget_tick(dt):
//!     pass
//!
//! @on_init
//! def setup():
//!     pass
//! ```

use pyo3::prelude::*;
use pyo3::types::PyDict;

/// `@on_event(EventClass)` — register a Python function as an event handler.
///
/// Returns a decorator (function → function) so it can be used as `@on_event(EarthquakeEvent)`.
#[pyfunction]
pub fn on_event(py: Python<'_>, event_type: &PyAny) -> PyResult<PyObject> {
    let event_type_name: String = event_type.getattr("__name__")?.extract()?;
    let registry = get_or_create_registry(py)?;

    // Return a decorator that takes the function and registers it
    let decorator = pyo3::types::PyCFunction::new_closure(
        py,
        None,
        None,
        move |args, _kwargs| {
            let py = args.py();
            let func: &PyAny = args.get_item(0)?;
            let handlers: &PyAny = registry
                .as_ref(py)
                .getattr("event_handlers")?;
            let entry = PyDict::new(py);
            entry.set_item("event_type", event_type_name.clone())?;
            entry.set_item("func", func)?;
            handlers.call_method1("append", (entry,))?;
            Ok::<_, pyo3::PyErr>(func.clone().into_py(py))
        },
    )?;

    Ok(decorator.into_py(py))
}

/// `@on_tick` or `@on_tick(rate_hz=4)` — register a function called each tick.
///
/// Supports both bare `@on_tick` and parameterized `@on_tick(rate_hz=4)`.
#[pyfunction]
#[pyo3(signature = (func_or_rate=None, *, rate_hz=0))]
pub fn on_tick(
    py: Python<'_>,
    func_or_rate: Option<&PyAny>,
    rate_hz: u32,
) -> PyResult<PyObject> {
    let registry = get_or_create_registry(py)?;

    if let Some(func) = func_or_rate {
        // Bare @on_tick (no args)
        register_tick_handler(py, &registry.as_ref(py), func, 0)?;
        Ok(func.clone().into_py(py))
    } else {
        // @on_tick(rate_hz=4) — return a decorator
        let decorator = pyo3::types::PyCFunction::new_closure(
            py,
            None,
            None,
            move |args, _kwargs| {
                let py = args.py();
                let func: &PyAny = args.get_item(0)?;
                let reg = get_or_create_registry(py)?;
                register_tick_handler(py, &reg.as_ref(py), func, rate_hz)?;
                Ok::<_, pyo3::PyErr>(func.clone().into_py(py))
            },
        )?;
        Ok(decorator.into_py(py))
    }
}

fn register_tick_handler(
    py: Python<'_>,
    registry: &PyAny,
    func: &PyAny,
    rate_hz: u32,
) -> PyResult<()> {
    let handlers = registry.getattr("tick_handlers")?;
    let entry = PyDict::new(py);
    let func_name: String = func.getattr("__name__")?.extract().unwrap_or_else(|_| "unknown".into());
    entry.set_item("name", func_name)?;
    entry.set_item("func", func)?;
    entry.set_item("rate_hz", rate_hz)?;
    handlers.call_method1("append", (entry,))?;
    Ok(())
}

/// `@on_init` — register a function called once after all scripts are loaded.
#[pyfunction]
pub fn on_init(py: Python<'_>, func: &PyAny) -> PyResult<PyObject> {
    let registry = get_or_create_registry(py)?;
    let handlers = registry.as_ref(py).getattr("init_handlers")?;
    let entry = PyDict::new(py);
    entry.set_item("func", func)?;
    handlers.call_method1("append", (entry,))?;
    Ok(func.clone().into_py(py))
}

/// Get or create the module-level `canopy._registry` object.
fn get_or_create_registry(py: Python<'_>) -> PyResult<PyObject> {
    let canopy = py.import("canopy")?;
    if let Ok(reg) = canopy.getattr("_registry") {
        if !reg.is_none() {
            return Ok(reg.into_py(py));
        }
    }
    // Create a fresh registry namespace
    let registry = PyDict::new(py);
    registry.set_item("systems", pyo3::types::PyList::empty(py))?;
    registry.set_item("event_handlers", pyo3::types::PyList::empty(py))?;
    registry.set_item("tick_handlers", pyo3::types::PyList::empty(py))?;
    registry.set_item("init_handlers", pyo3::types::PyList::empty(py))?;
    canopy.setattr("_registry", &registry)?;
    Ok(registry.into_py(py))
}
