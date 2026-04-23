//! `canopy-assets` — Asset runtime and .canasset format.
//!
//! # .canasset Binary Format
//!
//! `.canasset` is Canopy's proprietary packed asset format. Key design goals:
//!
//! - **Memory-mappable**: The file layout mirrors the in-memory layout so
//!   assets can be loaded with a single `mmap` call, zero copying.
//! - **Self-contained**: One file contains meshes (all LoD levels), collision,
//!   textures, materials, and metadata. No dependent file lookups at runtime.
//! - **Zero runtime parsing**: The header section maps directly to typed structs
//!   via `bytemuck::from_bytes`. Variable-length sections are accessed via
//!   offsets stored in the header.
//!
//! # File Layout
//!
//! ```text
//! ┌─────────────────────────────────┐
//! │ CanAssetHeader (fixed, 256 bytes)│  Magic + version + section table
//! ├─────────────────────────────────┤
//! │ SectionTable [n_sections × 16b] │  offset + size per section
//! ├─────────────────────────────────┤
//! │ Metadata section                │  JSON-encoded asset metadata
//! ├─────────────────────────────────┤
//! │ Mesh section (LoD 0)            │  vertex + index buffers, attribute layout
//! ├─────────────────────────────────┤
//! │ Mesh section (LoD 1)            │
//! ├─────────────────────────────────┤
//! │ Mesh section (LoD 2)            │
//! ├─────────────────────────────────┤
//! │ Mesh section (LoD 3)            │
//! ├─────────────────────────────────┤
//! │ Mesh section (shadow proxy)     │  Low-poly shadow caster
//! ├─────────────────────────────────┤
//! │ Collision section               │  Convex hulls for physics
//! ├─────────────────────────────────┤
//! │ Texture section(s)              │  BC7/ASTC compressed mipmaps
//! ├─────────────────────────────────┤
//! │ Material section                │  PBR material parameters
//! └─────────────────────────────────┘
//! ```

pub mod format;
pub mod handle;
pub mod server;
pub mod types;

pub use handle::{AssetId, Handle};
pub use server::AssetServer;
pub use types::{AudioClip, LodSet, Material, Mesh, Texture};

pub mod prelude {
    pub use super::handle::{AssetId, Handle};
    pub use super::server::AssetServer;
    pub use super::types::{AudioClip, LodSet, Material, Mesh, Texture, VertexLayout};
}
