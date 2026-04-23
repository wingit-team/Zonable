//! Asset type definitions — Mesh, Texture, Material, LodSet, AudioClip.

use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3, Vec4};

// ---------------------------------------------------------------------------
// Vertex layout
// ---------------------------------------------------------------------------

/// Interleaved vertex format used by all engine meshes.
///
/// Matches the GPU layout expected by canopy-renderer's vertex shaders.
/// `bytemuck::Pod` enables zero-copy upload via `wgpu::Buffer::map_write`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    /// World-space position
    pub position: [f32; 3],
    /// Surface normal (unit length)
    pub normal: [f32; 3],
    /// Tangent + handedness (w = 1 or -1 for bitangent flip)
    pub tangent: [f32; 4],
    /// Primary UV for albedo/normal map
    pub uv0: [f32; 2],
    /// Secondary UV for lightmaps
    pub uv1: [f32; 2],
    /// Vertex color (RGBA, used for building variation tinting)
    pub color: [f32; 4],
}

impl Vertex {
    pub const SIZE: usize = std::mem::size_of::<Self>();

    pub fn position(&self) -> Vec3 {
        Vec3::from_array(self.position)
    }

    pub fn normal(&self) -> Vec3 {
        Vec3::from_array(self.normal)
    }
}

/// Identifies what data is present in a vertex buffer — used by the renderer
/// to select the correct shader permutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexLayout {
    pub has_normals: bool,
    pub has_tangents: bool,
    pub has_uv0: bool,
    pub has_uv1: bool,
    pub has_vertex_color: bool,
}

impl VertexLayout {
    pub const FULL: Self = Self {
        has_normals: true,
        has_tangents: true,
        has_uv0: true,
        has_uv1: true,
        has_vertex_color: true,
    };

    pub const POSITION_ONLY: Self = Self {
        has_normals: false,
        has_tangents: false,
        has_uv0: false,
        has_uv1: false,
        has_vertex_color: false,
    };
}

// ---------------------------------------------------------------------------
// Mesh
// ---------------------------------------------------------------------------

/// A single mesh (one LoD level or shadow proxy).
///
/// Vertices and indices are stored as raw byte vecs so they can be directly
/// uploaded to GPU buffers without deserialization.
///
/// # LoD sets
///
/// A `LodSet` bundles multiple `Mesh` instances (lod0–lod3 + shadow).
/// The `LodSelector` in canopy-renderer picks which one to render.
#[derive(Debug, Clone)]
pub struct Mesh {
    /// Raw vertex bytes (Vec<Vertex> cast to bytes via bytemuck)
    pub vertices: Vec<u8>,
    /// Raw index bytes (Vec<u32> or Vec<u16> depending on index_format)
    pub indices: Vec<u8>,
    pub index_count: u32,
    pub vertex_count: u32,
    pub layout: VertexLayout,
    pub index_u32: bool, // true = u32 indices, false = u16
    /// Axis-aligned bounding box (for frustum culling)
    pub aabb_min: Vec3,
    pub aabb_max: Vec3,
}

impl Mesh {
    pub fn from_vertices_indices(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        use bytemuck::cast_slice;
        let aabb_min = vertices.iter().fold(Vec3::splat(f32::MAX), |a, v| a.min(v.position()));
        let aabb_max = vertices.iter().fold(Vec3::splat(f32::MIN), |a, v| a.max(v.position()));
        Self {
            vertex_count: vertices.len() as u32,
            index_count: indices.len() as u32,
            vertices: cast_slice(&vertices).to_vec(),
            indices: cast_slice(&indices).to_vec(),
            layout: VertexLayout::FULL,
            index_u32: true,
            aabb_min,
            aabb_max,
        }
    }

    /// Approximate memory usage in bytes.
    pub fn memory_bytes(&self) -> usize {
        self.vertices.len() + self.indices.len()
    }
}

// ---------------------------------------------------------------------------
// LoD set
// ---------------------------------------------------------------------------

/// A complete set of LoD meshes for one asset.
///
/// - `lod[0]` = highest quality (closest)
/// - `lod[3]` = lowest quality (farthest)
/// - `shadow` = shadow-casting proxy (typically even lower poly than lod3)
///
/// The LodSelector uses screen-space coverage to pick the right level.
/// Thresholds are configurable per-material in Phase 2; for now:
///
/// | LoD | Screen coverage |
/// |-----|----------------|
/// | 0   | > 20%          |
/// | 1   | 5–20%          |
/// | 2   | 1–5%           |
/// | 3   | < 1%           |
#[derive(Debug, Clone)]
pub struct LodSet {
    pub lods: Vec<Mesh>,   // lod[0] = highest quality
    pub shadow: Option<Mesh>,
}

impl LodSet {
    pub fn select_lod(&self, screen_coverage: f32) -> &Mesh {
        // Default thresholds — will be overridden by material config in Phase 2
        let idx = if screen_coverage > 0.20 { 0 }
            else if screen_coverage > 0.05 { 1 }
            else if screen_coverage > 0.01 { 2 }
            else { 3 };
        let clamped = idx.min(self.lods.len().saturating_sub(1));
        &self.lods[clamped]
    }
}

// ---------------------------------------------------------------------------
// Texture
// ---------------------------------------------------------------------------

/// Compression format of texture data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    /// Raw RGBA8 — uncompressed, used during development
    Rgba8,
    /// BC7 — high-quality block compression for desktop (DX11+, Vulkan, Metal)
    Bc7,
    /// ASTC 4x4 — for mobile/Apple GPU (also supported on Apple Silicon desktop)
    Astc4x4,
    /// BC5 — two-channel (RG) for normal maps (X+Y, reconstruct Z in shader)
    Bc5,
}

/// A single texture asset (one semantic: albedo, normal, roughness, etc.).
///
/// Mipmaps are stored as a flat byte buffer with the mip chain packed sequentially
/// from mip0 (largest) to mip_N (1×1 or smallest valid mip). The renderer uploads
/// each level individually during GPU resource creation.
#[derive(Debug, Clone)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub mip_count: u8,
    pub format: TextureFormat,
    /// Raw compressed texture data including all mips
    pub data: Vec<u8>,
    /// Byte offsets of each mip level within `data`
    pub mip_offsets: Vec<u32>,
}

impl Texture {
    pub fn memory_bytes(&self) -> usize {
        self.data.len()
    }
}

// ---------------------------------------------------------------------------
// Material
// ---------------------------------------------------------------------------

/// PBR material parameters stored in a .canasset.
///
/// All texture references are by `AssetId` — the `AssetServer` resolves them
/// at load time. In a .canasset these are embedded directly, so IDs are
/// internal identifiers pointing to texture sections within the same file.
#[derive(Debug, Clone)]
pub struct Material {
    pub name: String,
    /// Base color multiplier (tint)
    pub base_color: Vec4,
    /// Roughness scalar (0 = mirror, 1 = fully rough)
    pub roughness: f32,
    /// Metallic scalar
    pub metallic: f32,
    /// Emissive color + intensity
    pub emissive: Vec3,
    pub emissive_strength: f32,
    /// Alpha cutoff for masked transparency
    pub alpha_cutoff: f32,
    /// Whether this material is double-sided
    pub double_sided: bool,
    // Texture section indices (index into CanAsset.sections)
    pub albedo_section: Option<u32>,
    pub normal_section: Option<u32>,
    pub roughness_metal_section: Option<u32>,
    pub emissive_section: Option<u32>,
    pub ao_section: Option<u32>,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            base_color: Vec4::ONE,
            roughness: 0.5,
            metallic: 0.0,
            emissive: Vec3::ZERO,
            emissive_strength: 0.0,
            alpha_cutoff: 0.5,
            double_sided: false,
            albedo_section: None,
            normal_section: None,
            roughness_metal_section: None,
            emissive_section: None,
            ao_section: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Audio
// ---------------------------------------------------------------------------

/// PCM audio clip. Decoded from WAV/OGG at asset load time.
/// `cpal` consumes raw f32 PCM samples.
#[derive(Debug, Clone)]
pub struct AudioClip {
    pub sample_rate: u32,
    pub channels: u8,
    /// Interleaved f32 PCM samples [L, R, L, R, ...]
    pub samples: Vec<f32>,
    pub duration_seconds: f32,
}

impl AudioClip {
    pub fn memory_bytes(&self) -> usize {
        self.samples.len() * 4
    }
}
