//! Asset handles — typed weak references to loaded assets.
//!
//! An `AssetId` is a unique u64 assigned when an asset is first registered.
//! A `Handle<T>` carries the `AssetId` plus type information. Handles are
//! `Clone + Copy` — cheap to pass around. The `AssetServer` maps `AssetId → Arc<T>`.
//!
//! # Lifetime
//!
//! Handles do not keep assets alive (they're "weak"). The `AssetServer` owns
//! the `Arc<T>` and decides when to evict based on LRU policy and memory budget.
//! If you need to guarantee an asset stays loaded, upgrade to a strong reference
//! via `server.get(&handle)` which returns `Option<Arc<T>>`.

use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};

/// Globally unique asset identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(pub u64);

impl AssetId {
    /// Generate a new unique ID. Uses a global counter (thread-safe).
    pub fn new_unique() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    pub fn invalid() -> Self {
        Self(0)
    }

    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

/// Typed weak handle to an asset of type `T`.
///
/// ```rust
/// let handle: Handle<Mesh> = asset_server.load("buildings/tower.canasset");
/// // Later in a render system:
/// if let Some(mesh) = asset_server.get(&handle) {
///     // mesh: Arc<Mesh>
/// }
/// ```
pub struct Handle<T> {
    pub id: AssetId,
    _marker: PhantomData<fn() -> T>,
}

// Manual Clone/Copy because `#[derive]` requires `T: Clone` which is too restrictive
impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Handle<T> {}

impl<T> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Handle<{}>({})", std::any::type_name::<T>(), self.id.0)
    }
}

impl<T> Handle<T> {
    pub fn new(id: AssetId) -> Self {
        Self { id, _marker: PhantomData }
    }

    pub fn invalid() -> Self {
        Self::new(AssetId::invalid())
    }

    pub fn is_valid(&self) -> bool {
        self.id.is_valid()
    }

    /// Erase the type for storage in heterogeneous maps.
    pub fn untyped(&self) -> UntypedHandle {
        UntypedHandle {
            id: self.id,
            type_name: std::any::type_name::<T>(),
        }
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<T> Eq for Handle<T> {}

impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// Type-erased handle for maps that store handles of different types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UntypedHandle {
    pub id: AssetId,
    pub type_name: &'static str,
}
