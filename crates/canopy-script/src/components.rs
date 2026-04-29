//! Built-in component types exposed to Python.
//!
//! These are the core components Python game code uses. They are registered as
//! Python classes in the `canopy.components` submodule.

use crate::py_math::{PyColor, PyVec3, PyQuat};
use pyo3::prelude::*;

// ---------------------------------------------------------------------------
// Transform
// ---------------------------------------------------------------------------

/// Spatial transform component.
///
/// ```python
/// from canopy.components import Transform
/// from canopy import Vec3, Quat
///
/// t = Transform(position=Vec3(10, 0, 20))
/// t.position.y = 5.0
/// ```
#[pyclass(name = "Transform")]
#[derive(Debug, Clone)]
pub struct PyTransform {
    #[pyo3(get, set)]
    pub position: PyVec3,
    #[pyo3(get, set)]
    pub rotation: PyQuat,
    #[pyo3(get, set)]
    pub scale: PyVec3,
}

#[pymethods]
impl PyTransform {
    #[new]
    #[pyo3(signature = (position=None, rotation=None, scale=None))]
    pub fn new(
        position: Option<PyVec3>,
        rotation: Option<PyQuat>,
        scale: Option<PyVec3>,
    ) -> Self {
        Self {
            position: position.unwrap_or_else(PyVec3::zero),
            rotation: rotation.unwrap_or_else(PyQuat::identity),
            scale: scale.unwrap_or(PyVec3 { inner: glam::Vec3::ONE }),
        }
    }

    fn __repr__(&self) -> String {
        format!("Transform(position={}, rotation={})", self.position.__repr__(), self.rotation.__repr__())
    }
}

// ---------------------------------------------------------------------------
// Mesh reference
// ---------------------------------------------------------------------------

#[pyclass(name = "Mesh")]
#[derive(Debug, Clone)]
pub struct PyMeshRef {
    #[pyo3(get, set)]
    pub asset: String,
    #[pyo3(get, set)]
    pub lod_bias: f32,
    #[pyo3(get, set)]
    pub cast_shadow: bool,
    #[pyo3(get, set)]
    pub receive_shadow: bool,
}

#[pymethods]
impl PyMeshRef {
    #[new]
    #[pyo3(signature = (asset, lod_bias=0.0, cast_shadow=true, receive_shadow=true))]
    pub fn new(asset: String, lod_bias: f32, cast_shadow: bool, receive_shadow: bool) -> Self {
        Self { asset, lod_bias, cast_shadow, receive_shadow }
    }

    fn __repr__(&self) -> String {
        format!("Mesh('{}')", self.asset)
    }
}

// ---------------------------------------------------------------------------
// BuildingData
// ---------------------------------------------------------------------------

#[pyclass(name = "BuildingData")]
#[derive(Debug, Clone)]
pub struct PyBuildingData {
    #[pyo3(get, set)]
    pub zone_id: u32,
    #[pyo3(get, set)]
    pub capacity: u32,
    #[pyo3(get, set)]
    pub occupancy: u32,
    #[pyo3(get, set)]
    pub construction_progress: f32,
    #[pyo3(get, set)]
    pub health: f32,
}

#[pymethods]
impl PyBuildingData {
    #[new]
    #[pyo3(signature = (zone_id, capacity, occupancy=0, construction_progress=1.0, health=1.0))]
    pub fn new(
        zone_id: u32,
        capacity: u32,
        occupancy: u32,
        construction_progress: f32,
        health: f32,
    ) -> Self {
        Self { zone_id, capacity, occupancy, construction_progress, health }
    }

    /// Occupancy ratio [0.0, 1.0]
    pub fn occupancy_ratio(&self) -> f32 {
        if self.capacity == 0 { 0.0 } else { self.occupancy as f32 / self.capacity as f32 }
    }

    pub fn is_complete(&self) -> bool { self.construction_progress >= 1.0 }

    fn __repr__(&self) -> String {
        format!("BuildingData(zone={}, {}/{} occupants)", self.zone_id, self.occupancy, self.capacity)
    }
}

// ---------------------------------------------------------------------------
// Zone
// ---------------------------------------------------------------------------

#[pyclass(name = "Zone")]
#[derive(Debug, Clone)]
pub struct PyZone {
    #[pyo3(get, set)]
    pub zone_type: String,
    #[pyo3(get, set)]
    pub zone_id: u32,
    #[pyo3(get, set)]
    pub tier: u8,
    #[pyo3(get, set)]
    pub damage: f32,
}

#[pymethods]
impl PyZone {
    #[new]
    #[pyo3(signature = (zone_type, zone_id=0, tier=1, damage=0.0))]
    pub fn new(zone_type: String, zone_id: u32, tier: u8, damage: f32) -> Self {
        Self { zone_type, zone_id, tier, damage }
    }

    pub fn is_damaged(&self) -> bool { self.damage > 0.1 }
    pub fn is_destroyed(&self) -> bool { self.damage >= 0.95 }

    fn __repr__(&self) -> String {
        format!("Zone(type='{}', tier={}, damage={:.1}%)", self.zone_type, self.tier, self.damage * 100.0)
    }
}

// ---------------------------------------------------------------------------
// Physics
// ---------------------------------------------------------------------------

#[pyclass(name = "RigidBody")]
#[derive(Debug, Clone)]
pub struct PyRigidBody {
    #[pyo3(get, set)]
    pub body_type: String,
}

#[pymethods]
impl PyRigidBody {
    #[new]
    #[pyo3(signature = (body_type="dynamic".to_string()))]
    pub fn new(body_type: String) -> Self {
        Self { body_type }
    }

    fn __repr__(&self) -> String {
        format!("RigidBody(type='{}')", self.body_type)
    }
}

#[pyclass(name = "Collider")]
#[derive(Debug, Clone)]
pub struct PyCollider {
    #[pyo3(get, set)]
    pub shape: String,
    #[pyo3(get, set)]
    pub half_extents: PyVec3,
}

#[pymethods]
impl PyCollider {
    #[new]
    #[pyo3(signature = (shape="cuboid".to_string(), half_extents=None))]
    pub fn new(shape: String, half_extents: Option<PyVec3>) -> Self {
        Self {
            shape,
            half_extents: half_extents.unwrap_or(PyVec3 { inner: glam::Vec3::new(0.5, 0.5, 0.5) }),
        }
    }

    fn __repr__(&self) -> String {
        format!("Collider(shape='{}')", self.shape)
    }
}

// ---------------------------------------------------------------------------
// CanopyApp Python entry
// ---------------------------------------------------------------------------

pub fn register_components(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyTransform>()?;
    m.add_class::<PyMeshRef>()?;
    m.add_class::<PyBuildingData>()?;
    m.add_class::<PyZone>()?;
    m.add_class::<PyRigidBody>()?;
    m.add_class::<PyCollider>()?;
    Ok(())
}
