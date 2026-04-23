//! `.canasset` binary format — in-memory structures.
//!
//! These structs are `#[repr(C)]` + `bytemuck::Pod` so they can be written
//! directly from/to file bytes with no serialization overhead.
//!
//! # Writing (canopy-pipeline)
//!
//! `canopy-pipeline`'s `asset_packager.py` writes these structures using Python's
//! `struct.pack` with the same field layout. Any change here must be reflected there.
//!
//! # Reading (runtime)
//!
//! ```rust
//! let bytes = std::fs::read("building.canasset")?;
//! let header = CanAssetHeader::from_bytes(&bytes[..CanAssetHeader::SIZE]);
//! // Then use section offsets to access mesh, texture, material data
//! ```

use bytemuck::{Pod, Zeroable};

/// Magic bytes at offset 0 of every .canasset file.
pub const CANASSET_MAGIC: [u8; 8] = *b"CANASSET";

/// Current format version. Increment when format changes, not backward-compatible.
pub const CANASSET_VERSION: u32 = 1;

/// Maximum number of sections in a .canasset file.
pub const MAX_SECTIONS: usize = 32;

/// Section type identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SectionKind {
    Metadata = 0,
    MeshLod0 = 1,
    MeshLod1 = 2,
    MeshLod2 = 3,
    MeshLod3 = 4,
    MeshShadow = 5,
    Collision = 6,
    TextureAlbedo = 7,
    TextureNormal = 8,
    TextureRoughnessMetal = 9,  // Roughness in R, Metallic in G (packed)
    TextureEmissive = 10,
    TextureAO = 11,
    Material = 12,
    AudioClip = 13,
    // Reserved for future: animation, morph targets, physics materials, etc.
    Unknown = 0xFFFF_FFFF,
}

impl SectionKind {
    pub fn from_u32(v: u32) -> Self {
        match v {
            0 => Self::Metadata,
            1 => Self::MeshLod0,
            2 => Self::MeshLod1,
            3 => Self::MeshLod2,
            4 => Self::MeshLod3,
            5 => Self::MeshShadow,
            6 => Self::Collision,
            7 => Self::TextureAlbedo,
            8 => Self::TextureNormal,
            9 => Self::TextureRoughnessMetal,
            10 => Self::TextureEmissive,
            11 => Self::TextureAO,
            12 => Self::Material,
            13 => Self::AudioClip,
            _ => Self::Unknown,
        }
    }
}

/// Fixed-size file header. Exactly 64 bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CanAssetHeader {
    /// Must equal CANASSET_MAGIC
    pub magic: [u8; 8],
    /// Format version — CANASSET_VERSION
    pub version: u32,
    /// Number of sections following the header
    pub section_count: u32,
    /// Total file size in bytes (for validation)
    pub total_size: u64,
    /// Asset UUID (16 bytes) for content-addressed caching
    pub uuid: [u8; 16],
    /// Reserved padding to reach 64 bytes total
    pub _reserved: [u8; 24],
}

impl CanAssetHeader {
    pub const SIZE: usize = 64;

    pub fn new(uuid: [u8; 16], section_count: u32, total_size: u64) -> Self {
        Self {
            magic: CANASSET_MAGIC,
            version: CANASSET_VERSION,
            section_count,
            total_size,
            uuid,
            _reserved: [0u8; 24],
        }
    }

    /// Parse header from raw bytes. Returns None if magic or version mismatch.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        let header: &CanAssetHeader = bytemuck::from_bytes(&bytes[..Self::SIZE]);
        if header.magic != CANASSET_MAGIC {
            return None;
        }
        if header.version != CANASSET_VERSION {
            tracing::warn!(
                "canasset version mismatch: file={} engine={}",
                header.version, CANASSET_VERSION
            );
            return None;
        }
        Some(*header)
    }
}

/// Per-section descriptor in the section table (immediately after header).
/// Each entry is exactly 16 bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SectionEntry {
    /// `SectionKind as u32`
    pub kind: u32,
    /// Byte offset from start of file to section data
    pub offset: u32,
    /// Byte length of section data
    pub size: u32,
    /// Reserved (checksum or compression flags in future)
    pub flags: u32,
}

impl SectionEntry {
    pub const SIZE: usize = 16;

    pub fn kind(&self) -> SectionKind {
        SectionKind::from_u32(self.kind)
    }
}

/// In-memory representation of a loaded .canasset file.
///
/// The `data` field holds the raw bytes (either `Vec<u8>` from read or an `mmap`).
/// All section data is accessed via byte slices into `data` using offsets from
/// the section table — no copies needed after initial load.
pub struct CanAsset {
    pub header: CanAssetHeader,
    pub sections: Vec<SectionEntry>,
    /// Raw file bytes. Owned or memory-mapped (Arc<Mmap> in Phase 2).
    pub data: Vec<u8>,
}

impl CanAsset {
    /// Parse a .canasset from raw bytes.
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, AssetFormatError> {
        let header = CanAssetHeader::from_bytes(&data)
            .ok_or(AssetFormatError::InvalidMagic)?;

        let section_count = header.section_count as usize;
        let section_table_start = CanAssetHeader::SIZE;
        let section_table_size = section_count * SectionEntry::SIZE;

        if data.len() < section_table_start + section_table_size {
            return Err(AssetFormatError::TruncatedFile);
        }

        let section_bytes = &data[section_table_start..section_table_start + section_table_size];
        let sections: Vec<SectionEntry> = bytemuck::cast_slice(section_bytes).to_vec();

        Ok(Self { header, sections, data })
    }

    /// Get the raw bytes of a specific section.
    pub fn section_data(&self, kind: SectionKind) -> Option<&[u8]> {
        for entry in &self.sections {
            if entry.kind() == kind {
                let start = entry.offset as usize;
                let end = start + entry.size as usize;
                return self.data.get(start..end);
            }
        }
        None
    }

    /// Check whether a section of the given kind exists.
    pub fn has_section(&self, kind: SectionKind) -> bool {
        self.sections.iter().any(|e| e.kind() == kind)
    }

    /// Determine number of LoD levels available in this asset.
    pub fn lod_count(&self) -> u8 {
        let lods = [
            SectionKind::MeshLod0,
            SectionKind::MeshLod1,
            SectionKind::MeshLod2,
            SectionKind::MeshLod3,
        ];
        lods.iter().filter(|&&k| self.has_section(k)).count() as u8
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AssetFormatError {
    #[error("invalid magic bytes — not a .canasset file")]
    InvalidMagic,
    #[error("format version mismatch")]
    VersionMismatch,
    #[error("file is truncated")]
    TruncatedFile,
    #[error("section data out of bounds")]
    OutOfBounds,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
