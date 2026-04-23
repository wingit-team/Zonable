//! Camera — view/projection matrices and frustum.

use glam::{Mat4, Vec3};

#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    pub fov_y_radians: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn new(fov_degrees: f32, aspect: f32) -> Self {
        Self {
            position: Vec3::ZERO,
            forward: -Vec3::Z,
            up: Vec3::Y,
            fov_y_radians: fov_degrees.to_radians(),
            aspect,
            near: 0.1,
            far: 100_000.0, // City-builder needs large far plane
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_rh(self.position, self.forward, self.up)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        // Reversed-Z projection for better depth precision at large distances.
        // The renderer must be configured with DepthCompare::GreaterEqual when
        // using reversed-Z.
        Mat4::perspective_rh(self.fov_y_radians, self.aspect, self.far, self.near)
    }

    pub fn view_projection(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }
}
