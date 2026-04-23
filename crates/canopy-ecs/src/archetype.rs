//! Archetype system — groups entities by their exact component set.
//!
//! # Why Archetypes?
//!
//! Without archetypes, iterating over entities with components A+B requires scanning
//! *all* entities and checking "does this entity have A? does it have B?". With
//! archetypes, all entities sharing the same component set are grouped into one
//! archetype. A query for (A, B) only visits archetypes that contain both A and B —
//! no per-entity branching.
//!
//! # Layout
//!
//! An `ArchetypeId` is a sorted `Vec<ComponentId>` hashed to a `u64`. The `ArchetypeRegistry`
//! maps component-set → archetype metadata (which entities live there, etc.).
//!
//! In Phase 1 the archetype system is used for query filtering only — actual data
//! still lives in per-type `ComponentStorage<T>`. In Phase 2 we can migrate to
//! fully archetype-packed storage for SIMD hot paths.

use crate::component::ComponentId;
use crate::entity::Entity;
use ahash::{AHashMap, AHashSet};
use smallvec::SmallVec;

/// Sorted set of component IDs that defines an archetype.
/// `SmallVec<[ComponentId; 16]>` avoids heap allocation for entities
/// with ≤16 components (covers 99% of game entities).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArchetypeSignature(pub SmallVec<[ComponentId; 16]>);

impl ArchetypeSignature {
    pub fn new(mut ids: Vec<ComponentId>) -> Self {
        ids.sort_unstable();
        ids.dedup();
        Self(SmallVec::from_vec(ids))
    }

    pub fn contains(&self, id: ComponentId) -> bool {
        self.0.binary_search(&id).is_ok()
    }

    pub fn with(&self, id: ComponentId) -> Self {
        let mut ids = self.0.to_vec();
        if let Err(pos) = ids.binary_search(&id) {
            ids.insert(pos, id);
        }
        Self(SmallVec::from_vec(ids))
    }

    pub fn without(&self, id: ComponentId) -> Self {
        let mut ids = self.0.to_vec();
        if let Ok(pos) = ids.binary_search(&id) {
            ids.remove(pos);
        }
        Self(SmallVec::from_vec(ids))
    }
}

/// Numeric handle to a registered archetype. Used as a dense array index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArchetypeId(pub u32);

/// Metadata about a single archetype.
pub struct Archetype {
    pub id: ArchetypeId,
    pub signature: ArchetypeSignature,
    /// Entities currently in this archetype.
    pub entities: AHashSet<Entity>,
}

/// Global registry mapping component signatures → archetypes.
pub struct ArchetypeRegistry {
    archetypes: Vec<Archetype>,
    sig_to_id: AHashMap<ArchetypeSignature, ArchetypeId>,
    /// Track which archetype each live entity belongs to.
    entity_archetype: AHashMap<Entity, ArchetypeId>,
}

impl ArchetypeRegistry {
    pub fn new() -> Self {
        Self {
            archetypes: Vec::new(),
            sig_to_id: AHashMap::new(),
            entity_archetype: AHashMap::new(),
        }
    }

    /// Get or create an archetype for the given signature.
    pub fn get_or_create(&mut self, sig: ArchetypeSignature) -> ArchetypeId {
        if let Some(&id) = self.sig_to_id.get(&sig) {
            return id;
        }
        let id = ArchetypeId(self.archetypes.len() as u32);
        self.archetypes.push(Archetype {
            id,
            signature: sig.clone(),
            entities: AHashSet::new(),
        });
        self.sig_to_id.insert(sig, id);
        id
    }

    /// Move entity from its current archetype to a new one (after add/remove component).
    pub fn move_entity(&mut self, entity: Entity, new_sig: ArchetypeSignature) -> ArchetypeId {
        // Remove from old archetype
        if let Some(old_id) = self.entity_archetype.get(&entity).copied() {
            self.archetypes[old_id.0 as usize].entities.remove(&entity);
        }
        // Insert into new archetype
        let new_id = self.get_or_create(new_sig);
        self.archetypes[new_id.0 as usize].entities.insert(entity);
        self.entity_archetype.insert(entity, new_id);
        new_id
    }

    /// Remove an entity entirely (on despawn).
    pub fn remove_entity(&mut self, entity: Entity) {
        if let Some(arch_id) = self.entity_archetype.remove(&entity) {
            self.archetypes[arch_id.0 as usize].entities.remove(&entity);
        }
    }

    /// Get the current archetype of an entity.
    pub fn archetype_of(&self, entity: Entity) -> Option<ArchetypeId> {
        self.entity_archetype.get(&entity).copied()
    }

    /// Iterate over all archetypes whose signature is a superset of `required`.
    /// This is the core of query filtering — O(archetypes) which is typically small.
    pub fn matching_archetypes(
        &self,
        required: &[ComponentId],
    ) -> impl Iterator<Item = &Archetype> {
        self.archetypes
            .iter()
            .filter(|arch| required.iter().all(|req| arch.signature.contains(*req)))
    }

    pub fn get(&self, id: ArchetypeId) -> Option<&Archetype> {
        self.archetypes.get(id.0 as usize)
    }

    pub fn current_signature(&self, entity: Entity) -> Option<&ArchetypeSignature> {
        let arch_id = self.entity_archetype.get(&entity)?;
        Some(&self.archetypes[arch_id.0 as usize].signature)
    }
}

impl Default for ArchetypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
