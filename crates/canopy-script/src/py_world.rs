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
use canopy_ecs::world::World;
use pyo3::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;

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
    /// Components are stored per-entity keyed by Python type name.
    pub fn add(&self, py: Python<'_>, entity: &PyEntity, py_comp: PyObject) -> PyResult<()> {
        let entity_id = entity.to_entity();

        // 1. Handle builtin components by extracting native Rust equivalents
        // We do this BEFORE with_world to avoid borrow issues if possible, though extract is safe.
        let mut rust_transform = None;
        let mut rust_mesh = None;

        if let Ok(py_t) = py_comp.extract::<crate::components::PyTransform>(py) {
            rust_transform = Some(canopy_renderer::Transform {
                position: py_t.position.inner,
                rotation: py_t.rotation.inner,
                scale: py_t.scale.inner,
            });
        } else if let Ok(py_m) = py_comp.extract::<crate::components::PyMeshRef>(py) {
            rust_mesh = Some(canopy_renderer::MeshRef {
                asset: py_m.asset.clone(),
            });
        }

        let type_name = py_comp.as_ref(py).get_type().name()?.to_string();

        with_world(|w| {
            if let Some(t) = rust_transform {
                w.insert(entity_id, t);
            }
            if let Some(m) = rust_mesh {
                w.insert(entity_id, m);
            }

            // Keep Python-side components by type so entities can have Transform + Mesh + etc.
            if let Some(store) = w.get_mut::<PythonComponentStore>(entity_id) {
                store.components.insert(type_name, py_comp);
            } else {
                let mut components = HashMap::new();
                components.insert(type_name, py_comp);
                w.insert(entity_id, PythonComponentStore { components });
            }
        });

        Ok(())
    }

    /// Get a component from an entity by type name.
    ///
    /// ```python
    /// transform = world.get(entity, Transform)
    /// print(transform.position)
    /// ```
    pub fn get(&self, py: Python<'_>, entity: &PyEntity, component_type: &PyAny) -> PyResult<Option<PyObject>> {
        let type_name: String = component_type.getattr("__name__")?.extract()?;
        let result = with_world(|w| {
            w.get::<PythonComponentStore>(entity.to_entity())
                .and_then(|store| store.components.get(&type_name))
                .map(|obj| obj.clone_ref(py))
        });
        Ok(result)
    }

    /// Remove a component from an entity.
    pub fn remove(&self, entity: &PyEntity, component_type_name: &str) -> PyResult<()> {
        with_world(|w| {
            let entity_id = entity.to_entity();
            let should_remove_store = if let Some(store) = w.get_mut::<PythonComponentStore>(entity_id) {
                store.components.remove(component_type_name);
                store.components.is_empty()
            } else {
                false
            };
            if should_remove_store {
                w.remove::<PythonComponentStore>(entity_id);
            }
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
// Query API
// ---------------------------------------------------------------------------

#[pyclass(name = "Query")]
#[derive(Clone)]
pub struct PyQuery;

#[pymethods]
impl PyQuery {
    #[new]
    pub fn new() -> Self { Self }

    /// Iterate over entities having all requested components.
    ///
    /// ```python
    /// for entity, (transform, mesh) in query.with_components(Transform, Mesh):
    ///     pass
    /// ```
    #[pyo3(signature = (*args))]
    pub fn with_components(&self, py: Python<'_>, args: &pyo3::types::PyTuple) -> PyResult<PyObject> {
        let mut components = Vec::new();
        for arg in args.iter() {
            let type_name: String = arg.getattr("__name__")?.extract()?;
            components.push(type_name);
        }

        let results = pyo3::types::PyList::empty(py);
        
        with_world(|w| {
            let entities = w.query_filtered(&[
                canopy_ecs::component::ComponentId::of::<PythonComponentStore>(),
            ]);

            for entity in entities {
                let mut entity_components = Vec::new();
                let mut has_all = true;
                let Some(store) = w.get::<PythonComponentStore>(entity) else {
                    continue;
                };
                
                for type_name in &components {
                    if let Some(py_comp) = store.components.get(type_name) {
                        entity_components.push(py_comp.clone_ref(py));
                    } else {
                        has_all = false;
                        break;
                    }
                }
                
                if has_all {
                    let entity_py = PyEntity::from_entity(entity);
                    let comps_tuple = pyo3::types::PyTuple::new(py, &entity_components);
                    let row = pyo3::types::PyTuple::new(py, &[entity_py.into_py(py), comps_tuple.into_py(py)]);
                    let _ = results.append(row);
                }
            }
        });

        Ok(results.into_py(py))
    }
}

/// Helper for ScriptRunner to create a Query object.
pub fn canopy_script_query() -> PyQuery {
    PyQuery
}

// ---------------------------------------------------------------------------
// System Base Class
// ---------------------------------------------------------------------------

#[pyclass(name = "System", subclass)]
pub struct PySystemBase;

#[pymethods]
impl PySystemBase {
    #[new]
    pub fn new() -> Self { Self }

    pub fn on_tick(&self, _dt: f64, _query: PyQuery) -> PyResult<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Python component storage (Phase 1 — stores Python objects in the ECS)
// ---------------------------------------------------------------------------

/// Per-entity Python component storage.
///
/// Keys are Python class names (e.g. `Transform`, `Mesh`) and values are the
/// live Python objects for script-side mutation/query.
#[derive(Clone, Default)]
pub struct PythonComponentStore {
    pub components: HashMap<String, PyObject>,
}
