//! Entity identity — generation-packed u64 handles.
//!
//! An `Entity` is a 64-bit value split into two 32-bit halves:
//!
//! ```text
//! [  generation: u32  |  index: u32  ]
//!  63              32   31           0
//! ```
//!
//! **Why generations?** When an entity is destroyed and its slot reused, the
//! generation increments. Any old `Entity` handles pointing at the recycled slot
//! will have a stale generation and correctly fail lookup — no dangling references.
//!
//! We use `slotmap::SlotMap` internally because it implements exactly this pattern
//! with good performance. The `Entity` newtype wraps `slotmap::DefaultKey`.

use slotmap::{new_key_type, SlotMap};

new_key_type! {
    /// Opaque entity identifier. Internally a generation-tagged slot key.
    pub struct Entity;
}

/// Allocates and recycles entity IDs. Owned by `World`.
///
/// The allocator stores a `()` value per slot — we only care about the key.
/// Component data lives in per-type `ComponentStorage<T>` tables, not here.
pub struct EntityAllocator {
    slots: SlotMap<Entity, ()>,
}

impl EntityAllocator {
    pub fn new() -> Self {
        Self {
            slots: SlotMap::with_key(),
        }
    }

    /// Allocate a new entity ID. O(1) amortized.
    pub fn allocate(&mut self) -> Entity {
        self.slots.insert(())
    }

    /// Deallocate an entity. Returns `false` if the entity was already dead.
    pub fn free(&mut self, entity: Entity) -> bool {
        self.slots.remove(entity).is_some()
    }

    /// Check if an entity is currently alive.
    #[inline]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.slots.contains_key(entity)
    }

    /// Total number of live entities.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

impl Default for EntityAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_and_free() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.allocate();
        let e2 = alloc.allocate();
        assert!(alloc.is_alive(e1));
        assert!(alloc.is_alive(e2));
        assert!(alloc.free(e1));
        assert!(!alloc.is_alive(e1));
        // Double-free must return false
        assert!(!alloc.free(e1));
        assert_eq!(alloc.len(), 1);
    }

    #[test]
    fn generation_invalidates_old_handle() {
        let mut alloc = EntityAllocator::new();
        let e_old = alloc.allocate();
        alloc.free(e_old);
        let _e_new = alloc.allocate(); // may reuse the slot with new generation
        // The old handle must be dead even if the slot index was reused
        assert!(!alloc.is_alive(e_old));
    }
}
