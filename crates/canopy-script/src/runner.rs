//! `ScriptRunner` — manages Python system registration and dispatch.
//!
//! # Responsibilities
//!
//! 1. **Script loading**: At startup, scan `scripts_dir/` recursively, import each
//!    `.py` file, and execute it. Any Python class that subclasses `System` or
//!    function decorated with `@on_tick`/`@on_event`/`@on_init` is automatically
//!    registered.
//!
//! 2. **Frame dispatch**: Each frame, `run_frame(world, frame)` is called by
//!    `canopy-core`'s system scheduler. It:
//!    a. Sets the thread-local World pointer (so Python code can call `world.*`)
//!    b. Acquires the GIL
//!    c. Calls each registered Python system's `on_tick(dt)` method
//!    d. Releases the GIL
//!    e. Clears the World pointer
//!
//! 3. **Event dispatch**: When a `SimEvent` is published, `dispatch_event(event)`
//!    is called, which invokes all Python `@on_event` handlers for that event type.
//!
//! # GIL Optimization (Phase 2)
//!
//! Currently the GIL is held for the entire duration of Python system execution.
//! Phase 2 plan:
//! - Pre-collect query results (Vec<(Entity, ComponentSnapshot)>) WITHOUT the GIL
//! - Acquire GIL
//! - Pass pre-collected data to Python as lists (no World access needed during Python)
//! - Collect Python mutations as a list of commands
//! - Release GIL
//! - Apply mutations to World via EntityCommandBuffer

use canopy_ecs::world::World;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyString};
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};
use crate::py_world::{clear_active_world, set_active_world};

/// A registered Python system.
pub struct PySystem {
    pub name: String,
    pub instance: PyObject,
    /// Tick rate override from class attribute `tick_rate`. 0 = every frame.
    pub tick_rate_hz: u32,
    pub ticks_since_last_run: u32,
}

/// A registered Python event handler.
pub struct PyEventHandler {
    pub event_type_name: String,
    pub func: PyObject,
}

/// A registered Python @on_init function.
pub struct PyInitHandler {
    pub func: PyObject,
}

pub struct ScriptRunner {
    pub scripts_dir: PathBuf,
    pub systems: Vec<PySystem>,
    pub event_handlers: Vec<PyEventHandler>,
    pub init_handlers: Vec<PyInitHandler>,
    /// Target frame tick rate — used to compute tick_rate_hz ratios
    pub engine_tick_hz: u32,
}

impl ScriptRunner {
    pub fn new(scripts_dir: impl Into<PathBuf>, engine_tick_hz: u32) -> Self {
        Self {
            scripts_dir: scripts_dir.into(),
            systems: Vec::new(),
            event_handlers: Vec::new(),
            init_handlers: Vec::new(),
            engine_tick_hz,
        }
    }

    /// Scan and import all Python scripts in `scripts_dir`.
    ///
    /// Called once during engine startup. Scripts are imported in alphabetical order.
    /// `@on_init`, `@on_tick`, `@on_event` decorators in those scripts
    /// register handlers into the module-level registry (see decorators.rs).
    /// After import, we pull those registrations into this ScriptRunner.
    pub fn load_scripts(&mut self, py: Python<'_>) -> PyResult<()> {
        let scripts_dir = self.scripts_dir.clone();
        if !scripts_dir.exists() {
            warn!("scripts_dir {:?} does not exist — no Python scripts loaded", scripts_dir);
            return Ok(());
        }

        // Add scripts_dir to sys.path
        let sys = py.import("sys")?;
        let sys_path: &PyList = sys.getattr("path")?.downcast()?;
        sys_path.insert(0, scripts_dir.to_str().unwrap_or("."))?;

        info!("ScriptRunner: loading scripts from {:?}", scripts_dir);
        self.import_dir(py, &scripts_dir, "")?;

        // Pull registered systems from the global decorator registry
        self.sync_from_registry(py)?;

        Ok(())
    }

    fn import_dir(&self, py: Python<'_>, dir: &Path, prefix: &str) -> PyResult<()> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                let subprefix = if prefix.is_empty() {
                    entry.file_name().to_string_lossy().into_owned()
                } else {
                    format!("{}.{}", prefix, entry.file_name().to_string_lossy())
                };
                self.import_dir(py, &path, &subprefix)?;
            } else if path.extension().map_or(false, |e| e == "py") {
                let stem = path.file_stem().unwrap().to_string_lossy();
                if stem == "__init__" { continue; }
                let module_name = if prefix.is_empty() {
                    stem.into_owned()
                } else {
                    format!("{}.{}", prefix, stem)
                };
                info!("ScriptRunner: importing '{}'", module_name);
                if let Err(e) = py.import(module_name.as_str()) {
                    error!("ScriptRunner: failed to import '{}': {}", module_name, e);
                    // Don't abort — log and continue loading other scripts
                }
            }
        }
        Ok(())
    }

    fn sync_from_registry(&mut self, py: Python<'_>) -> PyResult<()> {
        // The decorator registry stores handlers in `canopy._registry` (set up in decorators.rs)
        let canopy = py.import("canopy")?;
        let registry = match canopy.getattr("_registry") {
            Ok(r) => r,
            Err(_) => return Ok(()), // No registry yet
        };

        // Sync systems
        if let Ok(systems) = registry.getattr("systems") {
            for item in systems.iter()? {
                let item = item?;
                let name: String = item.getattr("name")?.extract()?;
                let instance = item.getattr("instance")?.into();
                let tick_rate: u32 = item.getattr("tick_rate_hz")?.extract().unwrap_or(0);
                info!("ScriptRunner: registered system '{}'", name);
                self.systems.push(PySystem {
                    name,
                    instance,
                    tick_rate_hz: tick_rate,
                    ticks_since_last_run: 0,
                });
            }
        }

        // Sync event handlers
        if let Ok(handlers) = registry.getattr("event_handlers") {
            for item in handlers.iter()? {
                let item = item?;
                let event_type: String = item.getattr("event_type")?. extract()?;
                let func: PyObject = item.getattr("func")?.into();
                self.event_handlers.push(PyEventHandler {
                    event_type_name: event_type,
                    func,
                });
            }
        }

        Ok(())
    }

    /// Run all registered Python systems for this frame.
    ///
    /// Called by the ECS SystemScheduler as part of the `Update` stage.
    pub fn run_frame(&mut self, world: &mut World, dt: f64, frame_index: u64) {
        // Set the thread-local world pointer so Python can call world.*
        unsafe { set_active_world(world as *mut World); }

        Python::with_gil(|py| {
            for system in &mut self.systems {
                // Apply tick rate gating
                let should_run = if system.tick_rate_hz == 0 {
                    true // every frame
                } else {
                    let frames_per_tick = self.engine_tick_hz / system.tick_rate_hz.max(1);
                    system.ticks_since_last_run += 1;
                    if system.ticks_since_last_run >= frames_per_tick {
                        system.ticks_since_last_run = 0;
                        true
                    } else {
                        false
                    }
                };

                if !should_run { continue; }

                let result = system.instance
                    .as_ref(py)
                    .call_method1("on_tick", (dt,));

                if let Err(e) = result {
                    error!("ScriptRunner: error in system '{}': {}", system.name, e);
                    e.print(py);
                }
            }
        });

        clear_active_world();
    }

    /// Dispatch a sim event to all registered Python handlers for that event type.
    pub fn dispatch_event(&self, event_type: &str, event_data: &impl ToPyObject) {
        Python::with_gil(|py| {
            let py_event = event_data.to_object(py);
            for handler in &self.event_handlers {
                if handler.event_type_name == event_type {
                    if let Err(e) = handler.func.call1(py, (&py_event,)) {
                        error!("ScriptRunner: event handler error for '{}': {}", event_type, e);
                        e.print(py);
                    }
                }
            }
        });
    }

    /// Run all @on_init handlers. Called once after all scripts are loaded.
    pub fn run_init(&self, world: &mut World) {
        unsafe { set_active_world(world as *mut World); }
        Python::with_gil(|py| {
            for handler in &self.init_handlers {
                if let Err(e) = handler.func.call0(py) {
                    error!("ScriptRunner: @on_init error: {}", e);
                    e.print(py);
                }
            }
        });
        clear_active_world();
    }
}
