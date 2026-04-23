//! Python Entity wrapper.

use canopy_ecs::entity::Entity;
use pyo3::prelude::*;
use slotmap::KeyData;

/// Python-visible entity handle.
///
/// In Python:
/// ```python
/// entity = world.spawn()  # Returns a PyEntity
/// print(entity.id)        # numeric ID for debugging
/// ```
#[pyclass(name = "Entity")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PyEntity {
    /// The underlying ECS entity.
    /// Stored as u64 (the raw KeyData bits) for Python pickling / serialization.
    pub raw: u64,
}

impl PyEntity {
    pub fn from_entity(e: Entity) -> Self {
        // slotmap KeyData gives us the raw bits
        let data: KeyData = e.into();
        Self { raw: data.as_ffi() }
    }

    pub fn to_entity(&self) -> Entity {
        Entity::from(KeyData::from_ffi(self.raw))
    }
}

#[pymethods]
impl PyEntity {
    /// Numeric representation — useful for debugging, serialization.
    #[getter]
    pub fn id(&self) -> u64 { self.raw }

    pub fn is_valid(&self) -> bool { self.raw != 0 }

    fn __repr__(&self) -> String {
        format!("Entity({})", self.raw)
    }

    fn __hash__(&self) -> u64 {
        self.raw
    }

    fn __eq__(&self, other: &PyEntity) -> bool {
        self.raw == other.raw
    }
}
