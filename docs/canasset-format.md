# .canasset Binary Format Specification

The `.canasset` format is a proprietary, memory-mappable binary container used by the Canopy Engine. It is designed for zero-copy loading, where data can be uploaded directly to the GPU or accessed in memory without parsing steps.

## File Structure

A `.canasset` file consists of a fixed-size header, a variable-sized section table, and raw data blocks. All values are stored in **little-endian** format.

```text
+------------------------+  Offset 0
|  Header (64 bytes)     |
+------------------------+  Offset 64
|  Section Table         |
|  (N * 16 bytes)        |
+------------------------+  Offset (64 + N * 16)
|  Section Data Blocks   |
|  (Aligned to 16 bytes) |
+------------------------+
```

## 1. Header (64 bytes)

| Offset | Size | Type | Description |
| :--- | :--- | :--- | :--- |
| 0 | 8 | Bytes | Magic String: `b"CANASSET"` |
| 8 | 4 | u32 | Format Version (currently 1) |
| 12 | 4 | u32 | Section Count |
| 16 | 8 | u64 | Total File Size in bytes |
| 24 | 16 | Bytes | Unique Asset UUID |
| 40 | 24 | Bytes | Reserved (zeros) |

## 2. Section Table

Each entry in the section table is 16 bytes.

| Offset | Size | Type | Description |
| :--- | :--- | :--- | :--- |
| 0 | 4 | u32 | Section Kind (see below) |
| 4 | 4 | u32 | Offset from file start |
| 8 | 4 | u32 | Size of section data in bytes |
| 12 | 4 | u32 | Flags (reserved) |

### Section Kinds

| Value | Name | Description |
| :--- | :--- | :--- |
| 0 | Metadata | JSON encoded string for debugging/tooling |
| 1 | MeshLod0 | High-poly vertex and index data |
| 2-4 | MeshLod1-3 | Progressive LoD reductions |
| 5 | MeshShadow | Ultra-low poly shadow proxy |
| 6 | Collision | Convex hulls or triangle soup for physics |
| 7 | TextureAlbedo | Block-compressed (BC7/ASTC) albedo + mips |
| 8 | TextureNormal | Block-compressed (BC5) normal map + mips |
| 12 | Material | PBR parameters and texture handle references |
| 13 | AudioClip | PCM or Vorbis audio data |

## 3. Data Layouts

### Mesh Data Section
Mesh sections use an interleaved vertex layout for optimal cache performance.

**Mesh Header (48 bytes):**
- `vertex_count` (u32)
- `index_count` (u32)
- `index_format` (u32: 0=u16, 1=u32)
- `flags` (u32)
- `aabb_min` (f32 x 3)
- `aabb_max` (f32 x 3)
- `reserved` (u32)

**Vertex Structure (72 bytes):**
- `position` (f32 x 3)
- `normal` (f32 x 3)
- `tangent` (f32 x 4)
- `uv0` (f32 x 2)
- `uv1` (f32 x 2)
- `color` (f32 x 4)

### Texture Data Section
Packed texture blobs include a full mipmap chain.

**Texture Header:**
- `format_code` (u32: 1=BC7, 2=ASTC, 3=BC5)
- `width` (u32)
- `height` (u32)
- `mip_count` (u32)
- `mip_offsets` (u32 x mip_count): Offsets relative to the end of this header.

## Alignment Requirements
To ensure compatibility with memory-mapping and GPU alignment requirements, every section must start at an offset that is a multiple of **16 bytes**.
