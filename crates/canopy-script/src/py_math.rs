//! Python math types — Vec3, Quat, Color.
//!
//! These wrap `glam` types and expose them as Python classes with operator
//! overloads so Python code feels natural:
//!
//! ```python
//! pos = Vec3(1.0, 2.0, 3.0)
//! offset = Vec3(0.0, 10.0, 0.0)
//! new_pos = pos + offset          # __add__
//! dist = pos.distance(new_pos)    # method
//! normalized = pos.normalized()   # method
//! ```

use glam::{Quat, Vec3};
use pyo3::prelude::*;

// ---------------------------------------------------------------------------
// Vec3
// ---------------------------------------------------------------------------

#[pyclass(name = "Vec3")]
#[derive(Debug, Clone, Copy)]
pub struct PyVec3 {
    pub inner: Vec3,
}

#[pymethods]
impl PyVec3 {
    #[new]
    #[pyo3(signature = (x = 0.0, y = 0.0, z = 0.0))]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { inner: Vec3::new(x, y, z) }
    }

    #[staticmethod]
    pub fn zero() -> Self { Self { inner: Vec3::ZERO } }

    #[staticmethod]
    pub fn one() -> Self { Self { inner: Vec3::ONE } }

    #[staticmethod]
    pub fn up() -> Self { Self { inner: Vec3::Y } }

    #[staticmethod]
    pub fn forward() -> Self { Self { inner: -Vec3::Z } }

    #[getter]
    pub fn x(&self) -> f32 { self.inner.x }
    #[getter]
    pub fn y(&self) -> f32 { self.inner.y }
    #[getter]
    pub fn z(&self) -> f32 { self.inner.z }

    #[setter]
    pub fn set_x(&mut self, v: f32) { self.inner.x = v; }
    #[setter]
    pub fn set_y(&mut self, v: f32) { self.inner.y = v; }
    #[setter]
    pub fn set_z(&mut self, v: f32) { self.inner.z = v; }

    pub fn length(&self) -> f32 { self.inner.length() }

    pub fn normalized(&self) -> PyVec3 {
        Self { inner: self.inner.normalize_or_zero() }
    }

    pub fn distance(&self, other: &PyVec3) -> f32 {
        self.inner.distance(other.inner)
    }

    pub fn dot(&self, other: &PyVec3) -> f32 {
        self.inner.dot(other.inner)
    }

    pub fn cross(&self, other: &PyVec3) -> PyVec3 {
        Self { inner: self.inner.cross(other.inner) }
    }

    pub fn lerp(&self, other: &PyVec3, t: f32) -> PyVec3 {
        Self { inner: self.inner.lerp(other.inner, t) }
    }

    fn __add__(&self, other: &PyVec3) -> PyVec3 {
        Self { inner: self.inner + other.inner }
    }

    fn __sub__(&self, other: &PyVec3) -> PyVec3 {
        Self { inner: self.inner - other.inner }
    }

    fn __mul__(&self, scalar: f32) -> PyVec3 {
        Self { inner: self.inner * scalar }
    }

    fn __truediv__(&self, scalar: f32) -> PyVec3 {
        Self { inner: self.inner / scalar }
    }

    fn __neg__(&self) -> PyVec3 {
        Self { inner: -self.inner }
    }

    pub fn __repr__(&self) -> String {
        format!("Vec3({:.3}, {:.3}, {:.3})", self.inner.x, self.inner.y, self.inner.z)
    }

    fn __eq__(&self, other: &PyVec3) -> bool {
        self.inner == other.inner
    }

    /// Unpack as tuple for interop: `x, y, z = my_vec3`
    fn __iter__(&self) -> PyVecIter {
        PyVecIter { values: [self.inner.x, self.inner.y, self.inner.z], idx: 0 }
    }
}

#[pyclass]
struct PyVecIter {
    values: [f32; 3],
    idx: usize,
}

#[pymethods]
impl PyVecIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> { slf }
    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<f32> {
        if slf.idx < 3 { let v = slf.values[slf.idx]; slf.idx += 1; Some(v) } else { None }
    }
}

// ---------------------------------------------------------------------------
// Quat
// ---------------------------------------------------------------------------

#[pyclass(name = "Quat")]
#[derive(Debug, Clone, Copy)]
pub struct PyQuat {
    pub inner: Quat,
}

#[pymethods]
impl PyQuat {
    #[new]
    #[pyo3(signature = (x = 0.0, y = 0.0, z = 0.0, w = 1.0))]
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { inner: Quat::from_xyzw(x, y, z, w).normalize() }
    }

    #[staticmethod]
    pub fn identity() -> Self { Self { inner: Quat::IDENTITY } }

    #[staticmethod]
    pub fn from_axis_angle(axis: &PyVec3, angle_radians: f32) -> Self {
        Self { inner: Quat::from_axis_angle(axis.inner, angle_radians) }
    }

    #[staticmethod]
    pub fn from_euler_xyz(pitch: f32, yaw: f32, roll: f32) -> Self {
        Self { inner: Quat::from_euler(glam::EulerRot::XYZ, pitch, yaw, roll) }
    }

    pub fn rotate_vec(&self, v: &PyVec3) -> PyVec3 {
        PyVec3 { inner: self.inner * v.inner }
    }

    pub fn slerp(&self, other: &PyQuat, t: f32) -> PyQuat {
        Self { inner: self.inner.slerp(other.inner, t) }
    }

    fn __mul__(&self, other: &PyQuat) -> PyQuat {
        Self { inner: self.inner * other.inner }
    }

    pub fn __repr__(&self) -> String {
        format!("Quat({:.3}, {:.3}, {:.3}, {:.3})",
            self.inner.x, self.inner.y, self.inner.z, self.inner.w)
    }
}

// ---------------------------------------------------------------------------
// Color
// ---------------------------------------------------------------------------

#[pyclass(name = "Color")]
#[derive(Debug, Clone, Copy)]
pub struct PyColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[pymethods]
impl PyColor {
    #[new]
    #[pyo3(signature = (r = 1.0, g = 1.0, b = 1.0, a = 1.0))]
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self { Self { r, g, b, a } }

    #[staticmethod]
    pub fn white() -> Self { Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 } }
    #[staticmethod]
    pub fn black() -> Self { Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 } }
    #[staticmethod]
    pub fn red() -> Self { Self { r: 1.0, g: 0.0, b: 0.0, a: 1.0 } }
    #[staticmethod]
    pub fn from_hex(hex: &str) -> PyResult<Self> {
        let h = hex.trim_start_matches('#');
        if h.len() != 6 { return Err(pyo3::exceptions::PyValueError::new_err("hex must be 6 chars")); }
        let r = u8::from_str_radix(&h[0..2], 16).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))? as f32 / 255.0;
        let g = u8::from_str_radix(&h[2..4], 16).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))? as f32 / 255.0;
        let b = u8::from_str_radix(&h[4..6], 16).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))? as f32 / 255.0;
        Ok(Self { r, g, b, a: 1.0 })
    }

    #[getter] pub fn r(&self) -> f32 { self.r }
    #[getter] pub fn g(&self) -> f32 { self.g }
    #[getter] pub fn b(&self) -> f32 { self.b }
    #[getter] pub fn a(&self) -> f32 { self.a }

    pub fn __repr__(&self) -> String {
        format!("Color({:.2}, {:.2}, {:.2}, {:.2})", self.r, self.g, self.b, self.a)
    }
}
