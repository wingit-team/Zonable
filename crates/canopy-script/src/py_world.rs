//! Python World API — the primary interface for Python game code.
//!
//! The `PyWorld` class exposes ECS operations to Python. It holds a raw pointer
//! to the engine's `World` which is valid for the duration of the frame.
//!
//! # Thread Safety
//!
//! `PyWorld` is only accessible from the main thread (Python GIL is held).
//! The `ScriptRunner` guarantees the `World` pointer is valid before calling
//! any Python code.
//!
//! # Usage
//! ```python
//! from canopy import world, Vec3
//! from canopy.components import Transform
//!
//! entity = world.spawn()
//! world.add(entity, Transform(position=Vec3(10, 0, 20)))
//! transform = world.get(entity, Transform)
//! world.despawn(entity)
//! ```

use crate::py_entity::PyEntity;
use crate::py_math::PyVec3;
use canopy_ecs::world::World;
use pyo3::prelude::*;
use std::cell::RefCell;

/// Thread-local storage for the current frame's World pointer.
/// Set by `ScriptRunner::run_frame` before invoking Python, cleared after.
thread_local! {
    static CURRENT_WORLD: RefCell<Option<*mut World>> = RefCell::new(None);
}

/// Set the active world for this thread. Called by ScriptRunner.
///
/// # Safety
/// The pointer must be valid and not moved while Python code is executing.
pub unsafe fn set_active_world(world: *mut World) {
    CURRENT_WORLD.with(|w| {
        *w.borrow_mut() = Some(world);
    });
}

/// Clear the active world pointer. Called by ScriptRunner after Python returns.
pub fn clear_active_world() {
    CURRENT_WORLD.with(|w| {
        *w.borrow_mut() = None;
    });
}

/// Access the active world from Python callbacks. Panics if not set.
fn with_world<R>(f: impl FnOnce(&mut World) -> R) -> R {
    CURRENT_WORLD.with(|cell| {
        let ptr = cell.borrow()
            .expect("World not set — are you calling engine APIs outside of a system/tick?");
        // Safety: ScriptRunner guarantees exclusive access
        unsafe { f(&mut *ptr) }
    })
}

/// Python-facing World API.
///
/// This is a *view* object — it doesn't own the World, it just provides Python
/// access to the engine's World for the duration of the current frame.
#[pyclass(name = "World")]
pub struct PyWorld;

#[pymethods]
impl PyWorld {
    /// Spawn a new entity with no components.
    pub fn spawn(&self) -> PyEntity {
        let entity = with_world(|w| w.spawn());
        PyEntity::from_entity(entity)
    }

    /// Despawn an entity and remove all its components.
    pub fn despawn(&self, entity: &PyEntity) -> bool {
        with_world(|w| w.despawn(entity.to_entity()))
    }

    /// Check if an entity is alive.
    pub fn is_alive(&self, entity: &PyEntity) -> bool {
        with_world(|w| w.is_alive(entity.to_entity()))
    }

    /// Add (or overwrite) a component on an entity.
    ///
    /// The component must be a Python class registered via `register_components`.
    /// In Phase 1 we accept any Python object and store it as a `PyObject`.
    /// Phase 2 will use proper Rust component types for built-ins.
    pub fn add(&self, py: Python<'_>, entity: &PyEntity, component: PyObject) -> PyResult<()> {
        // Extract the component class name for the type registry
        let type_name: String = component
            .bind(py)
            .get_type()
            .name()?
            .to_string();

        with_world(|w| {
            // Store Python components as PyComponentStore entries
            // This inserts a Box<PyComponentEntry> into a special Python-component storage
            // that lives on the World as a resource (not a typed component).
            //
            // Phase 2: Built-in components (Transform, Mesh) will use native Rust types
            // and only user-defined Python components will use this path.
            if let Some(store) = w.get_mut::<PythonComponentStore>(canopy_ecs::entity::Entity::from(
                slotmap::KeyData::from_ffi(entity.raw)
            )) {
                // Store not needed here — we directly insert
            }
            // For Phase 1: store as a typed Python component
            let py_comp = PythonComponent {
                type_name: type_name.clone(),
                object: component.clone_ref(py),
            };
            w.insert(entity.to_entity(), py_comp);
        });
        Ok(())
    }

    /// Get a component from an entity by type name.
    ///
    /// ```python
    /// transform = world.get(entity, Transform)
    /// print(transform.position)
    /// ```
    pub fn get(&self, py: Python<'_>, entity: &PyEntity, component_type: &Bound<'_, PyAny>) -> PyResult<Option<PyObject>> {
        let type_name: String = component_type.get_type().name()?.to_string();
        // In Phase 1: look for the component by type name in PythonComponent storage
        // Phase 2: route built-in types to native storage
        let result = with_world(|w| {
            // TODO Phase 2: dispatch to native storage for Transform, Mesh, etc.
            w.get::<PythonComponent>(entity.to_entity())
                .filter(|c| c.type_name == type_name)
                .map(|c| c.object.clone_ref(py))
        });
        Ok(result)
    }

    /// Remove a component from an entity.
    pub fn remove(&self, entity: &PyEntity, component_type_name: &str) -> PyResult<()> {
        with_world(|w| {
            w.remove::<PythonComponent>(entity.to_entity());
        });
        Ok(())
    }

    /// Total number of live entities.
    pub fn entity_count(&self) -> usize {
        with_world(|w| w.entity_count())
    }

    /// Load a mesh asset and return an opaque handle integer.
    /// Full asset API exposed via `world.asset_server` in Phase 2.
    pub fn load_mesh(&self, path: &str) -> u64 {
        // Phase 2: call AssetServer.load_async(path), return Handle.id
        tracing::warn!("world.load_mesh('{}') — stub (Phase 2)", path);
        0
    }

    fn __repr__(&self) -> String {
        let count = with_world(|w| w.entity_count());
        format!("<World entities={}>", count)
    }
}

// ---------------------------------------------------------------------------
// Python component storage (Phase 1 — stores Python objects in the ECS)
// ---------------------------------------------------------------------------

/// A Python component stored in the ECS. Each Python `add(entity, obj)` call
/// creates one of these. Phase 2 will have per-type storage instead.
#[derive(Clone)]
pub struct PythonComponent {
    pub type_name: String,
    pub object: PyObject,
}

/// Marker type: entities can have multiple Python components of different types.
/// Phase 1 stores only the last-added component per entity (one PythonComponent slot).
/// Phase 2 will use a per-type-name HashMap keyed by type_name.
pub struct PythonComponentStore {
    // Map from type_name → PyObject
    components: std::collections::HashMap<String, PyObject>,
}
