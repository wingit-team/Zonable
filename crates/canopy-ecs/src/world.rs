//! `World` — the central ECS container.
//!
//! `World` owns:
//! - The `EntityAllocator` (entity IDs + generations)
//! - Per-type `ComponentStorage<T>` boxes (the actual data)
//! - The `ArchetypeRegistry` (tracks which archetype each entity belongs to)
//!
//! # Structural changes and the command buffer
//!
//! Spawning/despawning/add/remove during iteration would invalidate iterators.
//! Rule: **never call `world.spawn()` inside a query iteration loop**. Instead use
//! `EntityCommandBuffer` and flush it after the system completes. This is enforced
//! by the `SystemScheduler` which flushes the command buffer between stages.
//!
//! # Thread safety
//!
//! `World` is `!Send` and `!Sync` — it is owned exclusively by the main engine thread.
//! Read-only queries can be shared across rayon threads via scoped borrows (Phase 2).

use crate::archetype::{ArchetypeRegistry, ArchetypeSignature};
use crate::component::{Component, ComponentId};
use crate::entity::{Entity, EntityAllocator};
use crate::query::{ComponentIter, ComponentIterMut};
use crate::storage::{AnyStorage, ComponentStorage};
use ahash::AHashMap;
use std::any::TypeId;

pub struct World {
    pub(crate) allocator: EntityAllocator,
    /// Per-type erased storage. Keyed by `TypeId` of the component.
    pub(crate) storages: AHashMap<TypeId, Box<dyn AnyStorage>>,
    pub(crate) archetypes: ArchetypeRegistry,
}

impl World {
    pub fn new() -> Self {
        Self {
            allocator: EntityAllocator::new(),
            storages: AHashMap::new(),
            archetypes: ArchetypeRegistry::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Entity lifecycle
    // -----------------------------------------------------------------------

    /// Spawn a new entity with no components. Returns its ID.
    pub fn spawn(&mut self) -> Entity {
        let entity = self.allocator.allocate();
        // Move into the empty archetype (no components)
        self.archetypes.move_entity(entity, ArchetypeSignature::new(vec![]));
        entity
    }

    /// Despawn an entity and remove all its components.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.allocator.is_alive(entity) {
            return false;
        }
        // Remove from all storages
        for storage in self.storages.values_mut() {
            storage.remove_entity(entity);
        }
        self.archetypes.remove_entity(entity);
        self.allocator.free(entity);
        true
    }

    /// Returns true if the entity is alive.
    #[inline]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.allocator.is_alive(entity)
    }

    /// Number of live entities.
    pub fn entity_count(&self) -> usize {
        self.allocator.len()
    }

    // -----------------------------------------------------------------------
    // Component management
    // -----------------------------------------------------------------------

    /// Get or create the typed storage for component `T`.
    fn storage_mut<T: Component>(&mut self) -> &mut ComponentStorage<T> {
        self.storages
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(ComponentStorage::<T>::new()))
            .as_any_mut()
            .downcast_mut::<ComponentStorage<T>>()
            .expect("storage type mismatch — this is a bug")
    }

    fn storage<T: Component>(&self) -> Option<&ComponentStorage<T>> {
        self.storages.get(&TypeId::of::<T>()).and_then(|s| {
            s.as_any().downcast_ref::<ComponentStorage<T>>()
        })
    }

    /// Insert or overwrite a component on an entity.
    /// Updates the entity's archetype to include `T`.
    pub fn insert<T: Component>(&mut self, entity: Entity, value: T) {
        debug_assert!(self.allocator.is_alive(entity), "insert on dead entity");
        // Update archetype first
        let new_sig = self
            .archetypes
            .current_signature(entity)
            .cloned()
            .unwrap_or_else(|| ArchetypeSignature::new(vec![]))
            .with(ComponentId::of::<T>());
        self.archetypes.move_entity(entity, new_sig);
        // Insert into typed storage
        self.storage_mut::<T>().insert(entity, value);
    }

    /// Remove a component from an entity.
    /// Updates the entity's archetype to exclude `T`.
    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        if !self.allocator.is_alive(entity) {
            return None;
        }
        let new_sig = self
            .archetypes
            .current_signature(entity)
            .cloned()
            .unwrap_or_else(|| ArchetypeSignature::new(vec![]))
            .without(ComponentId::of::<T>());
        self.archetypes.move_entity(entity, new_sig);
        self.storage_mut::<T>().remove(entity)
    }

    /// Get a shared reference to a component.
    #[inline]
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.storage::<T>()?.get(entity)
    }

    /// Get a mutable reference to a component.
    #[inline]
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        self.storage_mut::<T>().get_mut(entity)
    }

    /// Returns `true` if the entity has component `T`.
    #[inline]
    pub fn has<T: Component>(&self, entity: Entity) -> bool {
        self.storage::<T>().map_or(false, |s| s.contains(entity))
    }

    // -----------------------------------------------------------------------
    // Queries — single component (safe, Phase 1)
    // -----------------------------------------------------------------------

    /// Iterate all (Entity, &T) pairs.
    pub fn query<T: Component>(&self) -> ComponentIter<'_, T> {
        match self.storage::<T>() {
            Some(s) => ComponentIter {
                entities: s.entities.iter(),
                data: s.data.iter(),
            },
            None => ComponentIter {
                entities: [].iter(),
                data: [].iter(),
            },
        }
    }

    /// Iterate all (Entity, &mut T) pairs.
    pub fn query_mut<T: Component>(&mut self) -> ComponentIterMut<'_, T> {
        let s = self.storage_mut::<T>();
        ComponentIterMut {
            entities: s.entities.iter(),
            data: s.data.iter_mut(),
        }
    }

    // -----------------------------------------------------------------------
    // Multi-component queries — filtered by archetype
    // -----------------------------------------------------------------------

    /// Collect all entities that have *all* of the listed component types.
    /// This is the archetype-filtered entity list used by high-level queries.
    ///
    /// Usage:
    /// ```rust
    /// let entities = world.query_filtered(&[
    ///     ComponentId::of::<Position>(),
    ///     ComponentId::of::<Velocity>(),
    /// ]);
    /// for entity in entities {
    ///     let pos = world.get_mut::<Position>(entity).unwrap();
    ///     let vel = world.get::<Velocity>(entity).unwrap();
    ///     pos.x += vel.dx;
    /// }
    /// ```
    ///
    /// This is intentionally a two-step API (get entity list, then get components)
    /// to avoid borrow-checker conflicts with simultaneous mutable borrows across
    /// multiple storage types. Phase 2 will use unsafe cell-based storage for
    /// true simultaneous mutable access.
    pub fn query_filtered(&self, required_components: &[ComponentId]) -> Vec<Entity> {
        self.archetypes
            .matching_archetypes(required_components)
            .flat_map(|arch| arch.entities.iter().copied())
            .collect()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct Health(f32);
    #[derive(Debug, Clone, PartialEq)]
    struct Speed(f32);

    #[test]
    fn spawn_and_insert() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Health(100.0));
        world.insert(e, Speed(5.0));
        assert_eq!(world.get::<Health>(e), Some(&Health(100.0)));
        assert_eq!(world.get::<Speed>(e), Some(&Speed(5.0)));
        assert_eq!(world.entity_count(), 1);
    }

    #[test]
    fn despawn_clears_components() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Health(50.0));
        assert!(world.despawn(e));
        assert!(!world.is_alive(e));
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn query_filtered_finds_matching() {
        let mut world = World::new();
        let e1 = world.spawn();
        world.insert(e1, Health(100.0));
        world.insert(e1, Speed(3.0));

        let e2 = world.spawn();
        world.insert(e2, Health(50.0));
        // e2 has no Speed

        let matches = world.query_filtered(&[
            ComponentId::of::<Health>(),
            ComponentId::of::<Speed>(),
        ]);

        assert_eq!(matches.len(), 1);
        assert!(matches.contains(&e1));
        assert!(!matches.contains(&e2));
    }

    #[test]
    fn query_single_iterates_all() {
        let mut world = World::new();
        for i in 0..10u32 {
            let e = world.spawn();
            world.insert(e, Health(i as f32));
        }
        let count = world.query::<Health>().count();
        assert_eq!(count, 10);
    }
}
