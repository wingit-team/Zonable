//! `canopy-script` — PyO3 Python scripting bindings.
//!
//! This crate is compiled as a `cdylib` (shared library) named `canopy.so` / `canopy.pyd`.
//! Python game code does `import canopy` and gets access to the full engine API.
//!
//! # Module Structure (Python)
//!
//! ```
//! canopy                    ← this crate
//! ├── CanopyApp             ← engine entry point
//! ├── EngineConfig          ← configuration
//! ├── Vec3, Quat, Color     ← math types
//! ├── Entity                ← entity handle
//! ├── World                 ← ECS world access
//! ├── System                ← base class for Python systems
//! ├── Query                 ← component query helper
//! ├── on_event              ← decorator: register event handler
//! ├── on_tick               ← decorator: register tick handler
//! ├── on_init               ← decorator: register init handler
//! └── components            ← built-in component types
//!     ├── Transform
//!     ├── Mesh
//!     ├── BuildingData
//!     └── Zone
//! ```
//!
//! # GIL Strategy
//!
//! Python systems run on the main thread inside `ScriptRunner::run_frame()`.
//! The `ScriptRunner` holds pre-gathered query results (collected without the GIL)
//! then acquires the GIL to call into Python with those results. This minimizes
//! GIL hold time. Full GIL-release optimization is Phase 2.
//!
//! # Panda3D Comparison
//!
//! | Panda3D          | Canopy          |
//! |------------------|-----------------|
//! | base.taskMgr.add | System.on_tick  |
//! | messenger.send   | on_event(...)   |
//! | loader.loadModel | world.load_mesh |
//! | render.attachNewNode | world.spawn + Transform |

use pyo3::prelude::*;

pub mod py_app;
pub mod py_entity;
pub mod py_math;
pub mod py_world;
pub mod components;
pub mod runner;
pub mod decorators;

use py_app::PyCanopyApp;
use py_entity::PyEntity;
use py_math::{PyColor, PyQuat, PyVec3};
use py_world::PyWorld;

/// The Python `canopy` module root.
///
/// Registered when Python does `import canopy`.
#[pymodule]
fn canopy(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Core types
    m.add_class::<PyCanopyApp>()?;
    m.add_class::<PyEntity>()?;
    m.add_class::<PyVec3>()?;
    m.add_class::<PyQuat>()?;
    m.add_class::<PyColor>()?;
    m.add_class::<PyWorld>()?;

    // Decorator functions
    m.add_function(wrap_pyfunction!(decorators::on_event, m)?)?;
    m.add_function(wrap_pyfunction!(decorators::on_tick, m)?)?;
    m.add_function(wrap_pyfunction!(decorators::on_init, m)?)?;

    // Components submodule
    let components_mod = PyModule::new(py, "components")?;
    components::register_components(py, &components_mod)?;
    m.add_submodule(&components_mod)?;

    // Convenience: expose `canopy.world` as a module-level singleton
    // (set by CanopyApp.run() before Python scripts are invoked)
    m.add("world", py.None())?;

    // Version info
    m.add("__version__", "0.1.0")?;
    m.add("__engine__", "Canopy Engine")?;

    Ok(())
}
