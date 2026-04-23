//! Dense SoA component storage.
//!
//! # Layout
//!
//! Each component type `T` gets its own `ComponentStorage<T>`. Internally this is:
//!
//! ```text
//! entities: Vec<Entity>          ← parallel array: which entity owns slot i
//! data:     Vec<T>               ← the actual component values (SoA)
//! sparse:   HashMap<Entity, u32> ← entity → dense index (the "sparse set" half)
//! ```
//!
//! This is a classic **sparse set** layout. It gives:
//! - O(1) insert, remove, and lookup by entity
//! - O(n) linear scan for iteration (cache-friendly for the data array)
//! - Swap-remove on delete keeps the dense arrays compact
//!
//! # Why not a raw `Vec<Option<T>>`?
//!
//! A flat array indexed by entity ID would waste enormous memory for a sparse world
//! (100k entities but only 5k have a given component). Sparse set gives compactness
//! while keeping iteration fast.

use crate::entity::Entity;
use ahash::AHashMap;
use std::any::Any;

/// Type-erased interface for component storage, used by `World` to store
/// heterogeneous storage boxes without knowing `T` at compile time.
pub trait AnyStorage: Any + Send + Sync {
    /// Remove the component for `entity` if present.
    fn remove_entity(&mut self, entity: Entity);
    /// Check if this storage has a component for `entity`.
    fn contains(&self, entity: Entity) -> bool;
    /// Downcast to `&dyn Any` for safe downcasting.
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Dense SoA storage for a single component type `T`.
pub struct ComponentStorage<T: 'static + Send + Sync> {
    /// Parallel arrays — index i corresponds to the same logical entry.
    pub entities: Vec<Entity>,
    pub data: Vec<T>,
    /// Sparse map: Entity → index into the dense arrays.
    sparse: AHashMap<Entity, u32>,
}

impl<T: 'static + Send + Sync> ComponentStorage<T> {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            data: Vec::new(),
            sparse: AHashMap::new(),
        }
    }

    /// Insert or overwrite a component for `entity`.
    pub fn insert(&mut self, entity: Entity, value: T) {
        if let Some(&idx) = self.sparse.get(&entity) {
            // Overwrite existing
            self.data[idx as usize] = value;
        } else {
            let idx = self.data.len() as u32;
            self.entities.push(entity);
            self.data.push(value);
            self.sparse.insert(entity, idx);
        }
    }

    /// Remove a component. Uses swap-remove to keep the arrays packed.
    /// Returns the removed value if it existed.
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let &idx = self.sparse.get(&entity)?;
        let idx = idx as usize;
        let last = self.data.len() - 1;

        // Swap with last element
        if idx != last {
            self.data.swap(idx, last);
            self.entities.swap(idx, last);
            // Update sparse entry for the swapped entity
            let swapped_entity = self.entities[idx];
            *self.sparse.get_mut(&swapped_entity).unwrap() = idx as u32;
        }

        self.sparse.remove(&entity);
        self.entities.pop();
        Some(self.data.pop().unwrap())
    }

    /// Get a shared reference to the component for `entity`.
    #[inline]
    pub fn get(&self, entity: Entity) -> Option<&T> {
        let &idx = self.sparse.get(&entity)?;
        Some(&self.data[idx as usize])
    }

    /// Get a mutable reference to the component for `entity`.
    #[inline]
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        let &idx = self.sparse.get(&entity)?;
        Some(&mut self.data[idx as usize])
    }

    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        self.sparse.contains_key(&entity)
    }

    /// Number of components stored.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Iterate over all (Entity, &T) pairs in dense order.
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.entities.iter().copied().zip(self.data.iter())
    }

    /// Iterate over all (Entity, &mut T) pairs in dense order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.entities.iter().copied().zip(self.data.iter_mut())
    }
}

impl<T: 'static + Send + Sync> Default for ComponentStorage<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static + Send + Sync> AnyStorage for ComponentStorage<T> {
    fn remove_entity(&mut self, entity: Entity) {
        self.remove(entity);
    }

    fn contains(&self, entity: Entity) -> bool {
        self.contains(entity)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;
    use slotmap::new_key_type;

    new_key_type! { struct TestEntity; }
    // Use Entity from crate for tests
    use crate::entity::{Entity, EntityAllocator};

    #[derive(Debug, PartialEq, Clone)]
    struct Hp(u32);

    #[test]
    fn insert_get_remove() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.allocate();
        let e2 = alloc.allocate();
        let mut storage = ComponentStorage::<Hp>::new();

        storage.insert(e1, Hp(100));
        storage.insert(e2, Hp(50));

        assert_eq!(storage.get(e1), Some(&Hp(100)));
        assert_eq!(storage.get(e2), Some(&Hp(50)));
        assert_eq!(storage.len(), 2);

        storage.remove(e1);
        assert_eq!(storage.get(e1), None);
        assert_eq!(storage.len(), 1);
        // e2 should still be accessible after swap-remove
        assert_eq!(storage.get(e2), Some(&Hp(50)));
    }

    #[test]
    fn overwrite() {
        let mut alloc = EntityAllocator::new();
        let e = alloc.allocate();
        let mut storage = ComponentStorage::<Hp>::new();
        storage.insert(e, Hp(10));
        storage.insert(e, Hp(999));
        assert_eq!(storage.get(e), Some(&Hp(999)));
        assert_eq!(storage.len(), 1);
    }
}
