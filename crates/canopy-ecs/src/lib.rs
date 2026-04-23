//! `canopy-ecs` — Canopy Engine's custom Entity Component System.
//!
//! # Design Goals
//!
//! - **Performance**: Dense SoA (Structure of Arrays) storage per component type.
//!   Iteration over a single component type is a linear scan — cache-friendly.
//! - **Archetype grouping**: Entities with the same set of components share an
//!   archetype, enabling SIMD-friendly inner-loop iteration over tightly packed data.
//! - **No runtime dispatch in the hot path**: Queries are monomorphized at compile time.
//! - **Parallelism-ready**: `Access<T>` markers on queries allow a future scheduler to
//!   detect read/write conflicts and run non-conflicting systems in parallel (rayon).
//!   Phase 1 executes single-threaded but the API contract is already correct.
//! - **Deferred mutations**: `EntityCommandBuffer` lets systems queue structural changes
//!   (spawn, despawn, add/remove component) without invalidating iterators mid-frame.
//!
//! # Example
//!
//! ```rust
//! use canopy_ecs::prelude::*;
//!
//! #[derive(Debug, Clone, Component)]
//! struct Position { x: f32, y: f32, z: f32 }
//!
//! #[derive(Debug, Clone, Component)]
//! struct Velocity { dx: f32, dy: f32, dz: f32 }
//!
//! let mut world = World::new();
//! let e = world.spawn();
//! world.insert(e, Position { x: 0.0, y: 0.0, z: 0.0 });
//! world.insert(e, Velocity { dx: 1.0, dy: 0.0, dz: 0.0 });
//!
//! // Query all entities with both Position and Velocity
//! for (entity, (pos, vel)) in world.query::<(&mut Position, &Velocity)>() {
//!     pos.x += vel.dx;
//!     pos.y += vel.dy;
//!     pos.z += vel.dz;
//! }
//! ```

pub mod archetype;
pub mod command;
pub mod component;
pub mod entity;
pub mod query;
pub mod storage;
pub mod system;
pub mod world;

// Re-export the derive macro from a companion proc-macro crate (to be added later).
// For now, `Component` is implemented manually or via the blanket below.
pub use component::Component;
pub use entity::Entity;
pub use world::World;

/// Convenience prelude — `use canopy_ecs::prelude::*;`
pub mod prelude {
    pub use super::command::EntityCommandBuffer;
    pub use super::component::Component;
    pub use super::entity::Entity;
    pub use super::query::{Access, Query, QueryIter};
    pub use super::system::{IntoSystem, System, SystemStage};
    pub use super::world::World;
    // Re-export glam so game code doesn't need a separate dep
    pub use glam::{Mat4, Quat, Vec2, Vec3, Vec3A, Vec4};
}
