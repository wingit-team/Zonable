//! `AssetServer` — async asset loading with LRU cache.
//!
//! # Responsibilities
//!
//! 1. **Path → AssetId mapping**: Canonicalize asset paths and assign stable IDs.
//! 2. **Async loading**: Fire-and-forget `load()` returns a `Handle<T>` immediately.
//!    A background Tokio task reads the file, parses it, and inserts into the cache.
//! 3. **LRU cache**: Keep recently-used assets in memory up to a budget (default 2 GB).
//!    Evict least-recently-used when over budget.
//! 4. **Boot LoD scan**: At startup `canopy-core` calls `scan_and_cache_lods()` which
//!    walks the assets directory, identifies meshes without LoD data, invokes
//!    `canopy-pipeline`'s LoD generator via subprocess or in-process Rust bindings,
//!    and writes the resulting `.canasset` back to disk.
//!
//! # Thread model
//!
//! `AssetServer` is `Send + Sync` — it wraps all mutable state in `Arc<Mutex<...>>`.
//! The main thread holds a clone; background Tokio tasks hold another clone.
//! Reads (game systems polling handles) take a read lock; writes (cache insert) take
//! a write lock but only on the HashMap, not on the asset data itself.

use crate::format::{CanAsset, CanAssetHeader, SectionKind};
use crate::handle::{AssetId, Handle, UntypedHandle};
use crate::types::{LodSet, Material, Mesh, Texture};
use ahash::AHashMap;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info, warn};

// ---------------------------------------------------------------------------
// Cache entry state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadState {
    /// Load requested but not yet started
    Queued,
    /// Currently loading from disk
    Loading,
    /// Successfully loaded
    Ready,
    /// Load failed with an error message
    Failed(String),
}

// ---------------------------------------------------------------------------
// Asset registry (path → AssetId + load state)
// ---------------------------------------------------------------------------

struct AssetEntry {
    id: AssetId,
    path: PathBuf,
    state: LoadState,
    /// Approximate bytes used in cache
    memory_bytes: usize,
    /// Last access epoch for LRU eviction
    last_used: u64,
}

// ---------------------------------------------------------------------------
// The server
// ---------------------------------------------------------------------------

struct ServerInner {
    root: PathBuf,
    /// Path (canonical) → registry entry
    registry: AHashMap<PathBuf, AssetEntry>,
    /// AssetId → loaded CanAsset (raw parsed file)
    loaded_canassets: AHashMap<AssetId, Arc<CanAsset>>,
    /// AssetId → extracted LodSet (typed, for renderers)
    lod_sets: AHashMap<AssetId, Arc<LodSet>>,
    /// Memory budget in bytes
    budget_bytes: usize,
    /// Current used bytes
    used_bytes: usize,
    /// Monotonic counter for LRU tracking
    epoch: u64,
}

#[derive(Clone)]
pub struct AssetServer {
    inner: Arc<RwLock<ServerInner>>,
}

impl AssetServer {
    /// Create a new server rooted at `assets_dir`.
    pub fn new(assets_dir: impl Into<PathBuf>, memory_budget_mb: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(ServerInner {
                root: assets_dir.into(),
                registry: AHashMap::new(),
                loaded_canassets: AHashMap::new(),
                lod_sets: AHashMap::new(),
                budget_bytes: memory_budget_mb * 1024 * 1024,
                used_bytes: 0,
                epoch: 0,
            })),
        }
    }

    /// Register a path and return a handle. The asset is NOT yet loaded.
    /// Call `load()` to start async loading, or `load_sync()` for blocking load.
    pub fn register(&self, path: impl Into<PathBuf>) -> Handle<CanAsset> {
        let path = path.into();
        let mut inner = self.inner.write();

        // Return existing handle if already registered
        if let Some(entry) = inner.registry.get(&path) {
            return Handle::new(entry.id);
        }

        let id = AssetId::new_unique();
        inner.registry.insert(path.clone(), AssetEntry {
            id,
            path,
            state: LoadState::Queued,
            memory_bytes: 0,
            last_used: 0,
        });
        Handle::new(id)
    }

    /// Synchronously load a `.canasset` file from disk.
    ///
    /// This is used during boot-time asset warm-up. During gameplay use
    /// `load_async()` to avoid blocking the game loop.
    pub fn load_sync(&self, path: impl AsRef<Path>) -> Result<Handle<CanAsset>, AssetError> {
        let path = path.as_ref();
        let handle = self.register(path.to_path_buf());

        // Check if already loaded
        {
            let inner = self.inner.read();
            let canonical = inner.root.join(path);
            if let Some(entry) = inner.registry.get(&canonical) {
                if entry.state == LoadState::Ready {
                    return Ok(handle);
                }
            }
        }

        let root = self.inner.read().root.clone();
        let full_path = root.join(path);
        let bytes = std::fs::read(&full_path)
            .map_err(|e| AssetError::Io(full_path.display().to_string(), e))?;

        let memory = bytes.len();
        let asset = CanAsset::from_bytes(bytes)
            .map_err(|e| AssetError::Format(e.to_string()))?;

        info!("AssetServer: loaded {:?} ({} KB)", path, memory / 1024);

        let mut inner = self.inner.write();
        inner.epoch += 1;
        let epoch = inner.epoch;

        // Update registry
        let canonical = inner.root.join(path);
        if let Some(entry) = inner.registry.get_mut(&canonical) {
            entry.state = LoadState::Ready;
            entry.memory_bytes = memory;
            entry.last_used = epoch;
        }

        inner.used_bytes += memory;
        inner.loaded_canassets.insert(handle.id, Arc::new(asset));

        // Evict if over budget
        if inner.used_bytes > inner.budget_bytes {
            // TODO Phase 2: LRU eviction — sort by last_used, remove oldest until under budget
            warn!("AssetServer: over memory budget ({} MB), LRU eviction not yet implemented",
                inner.used_bytes / 1024 / 1024);
        }

        Ok(handle)
    }

    /// Queue an async load. Returns a handle immediately — call `is_ready()` to poll.
    ///
    /// # Implementation (Phase 1 — blocking spawn)
    ///
    /// In Phase 1 we spawn a `std::thread` for simplicity. In Phase 2 this becomes
    /// a proper Tokio async task with a completion channel feeding back into the
    /// `LoadState` map on the next frame.
    pub fn load_async(&self, path: impl Into<PathBuf>) -> Handle<CanAsset> {
        let path = path.into();
        let handle = self.register(path.clone());
        let server = self.clone();

        std::thread::spawn(move || {
            if let Err(e) = server.load_sync(&path) {
                error!("AssetServer: async load failed for {:?}: {}", path, e);
                // Update state to Failed
                let mut inner = server.inner.write();
                if let Some(entry) = inner.registry.get_mut(&path) {
                    entry.state = LoadState::Failed(e.to_string());
                }
            }
        });

        handle
    }

    /// Get the load state of an asset.
    pub fn load_state(&self, handle: &Handle<CanAsset>) -> LoadState {
        let inner = self.inner.read();
        inner.registry.values()
            .find(|e| e.id == handle.id)
            .map(|e| e.state.clone())
            .unwrap_or(LoadState::Queued)
    }

    /// Get a loaded `CanAsset`, or `None` if not ready.
    pub fn get(&self, handle: &Handle<CanAsset>) -> Option<Arc<CanAsset>> {
        let mut inner = self.inner.write();
        inner.epoch += 1;
        let epoch = inner.epoch;
        // Touch last_used for LRU
        if let Some(entry) = inner.registry.values_mut().find(|e| e.id == handle.id) {
            entry.last_used = epoch;
        }
        inner.loaded_canassets.get(&handle.id).cloned()
    }

    /// Total loaded asset memory in bytes.
    pub fn used_memory_bytes(&self) -> usize {
        self.inner.read().used_bytes
    }

    /// Number of registered assets.
    pub fn registered_count(&self) -> usize {
        self.inner.read().registry.len()
    }
}

// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("io error loading '{0}': {1}")]
    Io(String, #[source] std::io::Error),
    #[error("format error: {0}")]
    Format(String),
    #[error("asset not found: {0}")]
    NotFound(String),
}
