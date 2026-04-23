//! Component trait and the global type-ID registry.
//!
//! # What is a Component?
//!
//! A component is a plain data struct (no logic, no virtual dispatch). The `Component`
//! trait is a marker — it just requires `'static + Send + Sync` so components can be
//! sent across threads safely and stored in `Any`-based maps.
//!
//! # Type Registry
//!
//! Each component type gets a stable `ComponentId` (a `u32` index into a global vec).
//! This is determined once at first use via `ComponentId::of::<T>()` and cached in a
//! `once_cell`-backed global. This lets us use `ComponentId` as a dense array index
//! in the archetype table, avoiding hash lookups in the hot iteration path.

use std::any::TypeId;
use std::sync::atomic::{AtomicU32, Ordering};
use ahash::AHashMap;
use parking_lot::RwLock;
use std::sync::OnceLock;

/// Every component type must implement this trait.
///
/// In practice, derive it: `#[derive(Component)]` (proc-macro crate, Phase 2).
/// For Phase 1, implement manually or use the blanket impl below.
pub trait Component: 'static + Send + Sync {
    /// Human-readable name for debugging and editor tooling.
    fn type_name() -> &'static str where Self: Sized {
        std::any::type_name::<Self>()
    }
}

/// A compact numeric ID for a component type. Cheaper than `TypeId` comparisons
/// and usable as a dense array index in the archetype bitmask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ComponentId(pub u32);

// ---------------------------------------------------------------------------
// Global component type registry
// ---------------------------------------------------------------------------

struct Registry {
    /// TypeId → ComponentId mapping
    map: AHashMap<TypeId, ComponentId>,
    /// Next ID to assign
    next: u32,
}

impl Registry {
    fn new() -> Self {
        Self {
            map: AHashMap::new(),
            next: 0,
        }
    }
}

static REGISTRY: OnceLock<RwLock<Registry>> = OnceLock::new();

fn global_registry() -> &'static RwLock<Registry> {
    REGISTRY.get_or_init(|| RwLock::new(Registry::new()))
}

impl ComponentId {
    /// Get or create the `ComponentId` for type `T`.
    ///
    /// The first call for each type acquires a write lock to insert a new entry.
    /// Subsequent calls use a read lock only. In practice the write path only fires
    /// during startup/level-load, never in the simulation hot loop.
    pub fn of<T: Component>() -> ComponentId {
        let type_id = TypeId::of::<T>();
        {
            // Fast path — already registered
            let reg = global_registry().read();
            if let Some(&id) = reg.map.get(&type_id) {
                return id;
            }
        }
        // Slow path — register new type
        let mut reg = global_registry().write();
        // Double-check after acquiring write lock
        if let Some(&id) = reg.map.get(&type_id) {
            return id;
        }
        let id = ComponentId(reg.next);
        reg.next += 1;
        reg.map.insert(type_id, id);
        id
    }

    pub fn as_index(self) -> usize {
        self.0 as usize
    }
}

// ---------------------------------------------------------------------------
// Blanket implementation
// ---------------------------------------------------------------------------
// Any type that is 'static + Send + Sync is a valid Component.
// This lets plain structs be used without the derive macro during Phase 1.
// The proc-macro will override this with additional metadata in Phase 2.

impl<T: 'static + Send + Sync> Component for T {}

#[cfg(test)]
mod tests {
    use super::*;

    struct Pos { x: f32 }
    struct Vel { dx: f32 }

    #[test]
    fn ids_are_stable() {
        let id1 = ComponentId::of::<Pos>();
        let id2 = ComponentId::of::<Pos>();
        assert_eq!(id1, id2);
    }

    #[test]
    fn different_types_get_different_ids() {
        let pos_id = ComponentId::of::<Pos>();
        let vel_id = ComponentId::of::<Vel>();
        assert_ne!(pos_id, vel_id);
    }
}
