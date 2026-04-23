//! Query system — compile-time typed iteration over component sets.
//!
//! # Query Design
//!
//! `world.query::<(&A, &mut B)>()` returns an iterator over all entities
//! that have both A and B. The type signature encodes access mode:
//! - `&T` → read-only
//! - `&mut T` → read-write
//!
//! # Access Markers (Parallelism Contract)
//!
//! The `WorldAccess` struct tracks which component types are read vs written
//! by a query. This is the information a future parallel scheduler needs to
//! detect conflicts between systems. In Phase 1 we construct this descriptor
//! but use it only for assertion checks. In Phase 2, the `SystemScheduler` uses
//! it to build a dependency graph and dispatch non-conflicting systems via rayon.
//!
//! # Implementation Strategy
//!
//! Rust's type system can't easily express "iterate over multiple heterogeneous
//! storage types simultaneously" without unsafe pointer aliasing. The safe approach
//! used here is:
//!
//! 1. Build a list of matching entity IDs (from archetypes).
//! 2. For each entity, do per-storage lookups to construct the tuple.
//!
//! This is slightly slower than archetype-packed storage (Phase 2 target) but is
//! safe, correct, and fast enough for Phase 1 (well under 100k entities).

use crate::component::{Component, ComponentId};
use crate::entity::Entity;
use std::marker::PhantomData;

// ---------------------------------------------------------------------------
// Access descriptors — used by the scheduler in Phase 2
// ---------------------------------------------------------------------------

/// Describes what component types a query reads and writes.
#[derive(Debug, Default, Clone)]
pub struct WorldAccess {
    pub reads: Vec<ComponentId>,
    pub writes: Vec<ComponentId>,
}

impl WorldAccess {
    pub fn read<T: Component>(&mut self) {
        self.reads.push(ComponentId::of::<T>());
    }

    pub fn write<T: Component>(&mut self) {
        self.writes.push(ComponentId::of::<T>());
    }

    /// Returns `true` if `other` has write access to something we read or write,
    /// or vice versa. Used to detect scheduling conflicts.
    pub fn conflicts_with(&self, other: &WorldAccess) -> bool {
        // Any shared write is a conflict
        for &w in &self.writes {
            if other.reads.contains(&w) || other.writes.contains(&w) {
                return true;
            }
        }
        for &w in &other.writes {
            if self.reads.contains(&w) {
                return true;
            }
        }
        false
    }
}

// ---------------------------------------------------------------------------
// QueryItem trait — implemented by reference types (&T, &mut T)
// ---------------------------------------------------------------------------

/// Implemented by types that can be fetched from a World for a single entity.
/// This is the building block for tuple queries.
///
/// # Safety
/// Implementors must not produce aliasing mutable references. The `World`
/// ensures this by only allowing one `&mut ComponentStorage<T>` at a time.
pub trait QueryItem<'w>: Sized {
    type Component: Component;

    /// Which access mode does this item need?
    fn register_access(access: &mut WorldAccess);
}

impl<'w, T: Component> QueryItem<'w> for &'w T {
    type Component = T;
    fn register_access(access: &mut WorldAccess) {
        access.read::<T>();
    }
}

impl<'w, T: Component> QueryItem<'w> for &'w mut T {
    type Component = T;
    fn register_access(access: &mut WorldAccess) {
        access.write::<T>();
    }
}

// ---------------------------------------------------------------------------
// QueryResult — what a query yields per entity
// ---------------------------------------------------------------------------

/// A single result from a query: the matching entity + its component data.
pub struct QueryResult<T> {
    pub entity: Entity,
    pub components: T,
}

// ---------------------------------------------------------------------------
// Concrete query iterator (single-component for now; tuples below)
// ---------------------------------------------------------------------------

/// Iterator returned by `World::query_one::<T>()`. Yields `(Entity, &T)` or
/// `(Entity, &mut T)` for all entities that have component T.
///
/// Full multi-component tuple queries are in `world.rs` using this as a primitive.
pub struct ComponentIter<'w, T: Component> {
    pub(crate) entities: std::slice::Iter<'w, Entity>,
    pub(crate) data: std::slice::Iter<'w, T>,
}

impl<'w, T: Component> Iterator for ComponentIter<'w, T> {
    type Item = (Entity, &'w T);

    fn next(&mut self) -> Option<Self::Item> {
        let entity = *self.entities.next()?;
        let data = self.data.next()?;
        Some((entity, data))
    }
}

pub struct ComponentIterMut<'w, T: Component> {
    pub(crate) entities: std::slice::Iter<'w, Entity>,
    pub(crate) data: std::slice::IterMut<'w, T>,
}

impl<'w, T: Component> Iterator for ComponentIterMut<'w, T> {
    type Item = (Entity, &'w mut T);

    fn next(&mut self) -> Option<Self::Item> {
        let entity = *self.entities.next()?;
        let data = self.data.next()?;
        Some((entity, data))
    }
}

/// Public alias re-exported in prelude for ergonomic use in system signatures.
pub type QueryIter<'w, T> = ComponentIter<'w, T>;

/// Placeholder type for the high-level multi-component query builder.
/// The actual query execution is driven by `World::query_filtered`.
/// See `world.rs` for the implementation.
pub struct Query<F>(PhantomData<F>);
