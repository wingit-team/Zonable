use rapier3d::prelude::*;
use glam::Vec3;

#[derive(Debug, Clone, PartialEq)]
pub enum RigidBodyType {
    Dynamic,
    Fixed,
}

#[derive(Debug, Clone)]
pub struct RigidBodyDesc {
    pub body_type: RigidBodyType,
}

#[derive(Debug, Clone)]
pub enum ColliderShape {
    Cuboid { half_extents: Vec3 },
}

#[derive(Debug, Clone)]
pub struct ColliderDesc {
    pub shape: ColliderShape,
}

/// Stores the actual Rapier handles so we can sync them.
#[derive(Debug, Clone)]
pub struct PhysicsHandles {
    pub body_handle: RigidBodyHandle,
    pub collider_handle: Option<ColliderHandle>,
}
