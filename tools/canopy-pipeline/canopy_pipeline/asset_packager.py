"""Asset packager — writes .canasset binary files.

# .canasset Format Reference

The format is defined in Rust at `crates/canopy-assets/src/format.rs`.
This Python writer must produce byte-identical output.

## File Layout

```
Offset 0:   CanAssetHeader (64 bytes)
Offset 64:  SectionTable [section_count × 16 bytes]
Offset 64 + section_count × 16:  Section data (variable)
```

## CanAssetHeader (64 bytes, little-endian)
- [8B]  magic = b'CANASSET'
- [4B]  version = 1
- [4B]  section_count
- [8B]  total_size
- [16B] uuid
- [24B] reserved (zeros)

## SectionEntry (16 bytes, little-endian)
- [4B]  kind (u32 from SectionKind enum)
- [4B]  offset (from file start)
- [4B]  size
- [4B]  flags (reserved, 0)

## Section Kinds (from format.rs)
0  Metadata
1  MeshLod0
2  MeshLod1
3  MeshLod2
4  MeshLod3
5  MeshShadow
6  Collision
7  TextureAlbedo
8  TextureNormal
9  TextureRoughnessMetal
10 TextureEmissive
11 TextureAO
12 Material
13 AudioClip
"""

from __future__ import annotations

import json
import logging
import os
import struct
import uuid
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path
from typing import Any

log = logging.getLogger(__name__)

# ── Format constants (mirror format.rs) ────────────────────────────────────
CANASSET_MAGIC = b"CANASSET"
CANASSET_VERSION = 1
HEADER_SIZE = 64
SECTION_ENTRY_SIZE = 16

class SectionKind:
    METADATA = 0
    MESH_LOD0 = 1
    MESH_LOD1 = 2
    MESH_LOD2 = 3
    MESH_LOD3 = 4
    MESH_SHADOW = 5
    COLLISION = 6
    TEXTURE_ALBEDO = 7
    TEXTURE_NORMAL = 8
    TEXTURE_ROUGHNESS_METAL = 9
    TEXTURE_EMISSIVE = 10
    TEXTURE_AO = 11
    MATERIAL = 12
    AUDIO_CLIP = 13

    NAMES = {
        0: "Metadata", 1: "MeshLod0", 2: "MeshLod1", 3: "MeshLod2",
        4: "MeshLod3", 5: "MeshShadow", 6: "Collision", 7: "TextureAlbedo",
        8: "TextureNormal", 9: "TextureRoughnessMetal", 10: "TextureEmissive",
        11: "TextureAO", 12: "Material", 13: "AudioClip",
    }

    @classmethod
    def name(cls, kind: int) -> str:
        return cls.NAMES.get(kind, f"Unknown({kind})")


# ── Low-level writer ────────────────────────────────────────────────────────

@dataclass
class Section:
    kind: int
    data: bytes

    @property
    def size(self) -> int:
        return len(self.data)


class CanAssetWriter:
    """Writes a .canasset file from a list of Section objects.

    Usage:
        writer = CanAssetWriter()
        writer.add_section(Section(SectionKind.MESH_LOD0, mesh_bytes))
        writer.add_section(Section(SectionKind.TEXTURE_ALBEDO, tex_bytes))
        writer.write("output.canasset")
    """

    def __init__(self):
        self.sections: list[Section] = []
        self._uuid = uuid.uuid4()

    def add_section(self, section: Section) -> None:
        self.sections.append(section)

    def add_metadata(self, meta: dict) -> None:
        self.add_section(Section(SectionKind.METADATA, json.dumps(meta).encode()))

    def write(self, output_path: str) -> int:
        """Write the .canasset file. Returns total bytes written."""
        section_count = len(self.sections)
        section_table_size = section_count * SECTION_ENTRY_SIZE

        # Compute section offsets
        data_start = HEADER_SIZE + section_table_size
        offsets: list[int] = []
        cursor = data_start
        for section in self.sections:
            # Align each section to 16 bytes for mmap-friendly access
            aligned = (cursor + 15) & ~15
            offsets.append(aligned)
            cursor = aligned + section.size

        total_size = cursor

        # Build header
        uuid_bytes = self._uuid.bytes
        header = struct.pack(
            "<8sIIQ16s24s",
            CANASSET_MAGIC,
            CANASSET_VERSION,
            section_count,
            total_size,
            uuid_bytes,
            b"\x00" * 24,
        )
        assert len(header) == HEADER_SIZE

        # Build section table
        section_table = b""
        for i, (section, offset) in enumerate(zip(self.sections, offsets)):
            entry = struct.pack("<IIII", section.kind, offset, section.size, 0)
            assert len(entry) == SECTION_ENTRY_SIZE
            section_table += entry

        # Write file
        output_path = Path(output_path)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        with open(output_path, "wb") as f:
            f.write(header)
            f.write(section_table)
            # Write sections at their computed offsets
            for section, offset in zip(self.sections, offsets):
                current = f.tell()
                if current < offset:
                    f.write(b"\x00" * (offset - current))
                f.write(section.data)

            # Pad to total_size
            current = f.tell()
            if current < total_size:
                f.write(b"\x00" * (total_size - current))

        log.info("Wrote %s (%d sections, %.1f KB)", output_path, section_count, total_size / 1024)
        return total_size


# ── Mesh encoding ───────────────────────────────────────────────────────────

def encode_mesh(mesh_data) -> bytes:
    """Encode a MeshData object to the binary format expected by canopy-assets.

    Vertex layout (must match Rust `Vertex` struct in types.rs):
    - position:  [f32; 3]
    - normal:    [f32; 3]
    - tangent:   [f32; 4]  (computed via cross product: tangent + handedness)
    - uv0:       [f32; 2]
    - uv1:       [f32; 2]  (same as uv0 for single-UV meshes)
    - color:     [f32; 4]  (default: 1.0, 1.0, 1.0, 1.0)

    Header (48 bytes):
    [4B] vertex_count
    [4B] index_count
    [4B] index_format (0=u16, 1=u32)
    [4B] flags
    [6×4B] aabb_min (x,y,z), aabb_max (x,y,z)
    [4B] reserved
    """
    import numpy as np

    verts = mesh_data.vertices   # (N, 3)
    norms = mesh_data.normals    # (N, 3)
    uvs = mesh_data.uvs          # (N, 2)
    indices = mesh_data.indices  # (M,)

    n = len(verts)
    # Compute tangents (flat, no UV-based computation in Phase 1)
    tangents = np.tile([1.0, 0.0, 0.0, 1.0], (n, 1)).astype(np.float32)
    uv1 = uvs.copy()
    colors = np.ones((n, 4), dtype=np.float32)

    # Interleave: position(3) + normal(3) + tangent(4) + uv0(2) + uv1(2) + color(4) = 18 floats
    vertex_data = np.concatenate([verts, norms, tangents, uvs, uv1, colors], axis=1)
    assert vertex_data.shape[1] == 18

    aabb_min = verts.min(axis=0)
    aabb_max = verts.max(axis=0)

    use_u32 = len(indices) > 65535
    idx_data = indices.astype(np.uint32 if use_u32 else np.uint16).tobytes()

    header = struct.pack(
        "<IIII6fI",
        n,               # vertex_count
        len(indices),    # index_count
        1 if use_u32 else 0,  # index_format
        0,               # flags
        *aabb_min,       # aabb_min x,y,z
        *aabb_max,       # aabb_max x,y,z
        0,               # reserved
    )

    return header + vertex_data.astype(np.float32).tobytes() + idx_data


# ── High-level packager ─────────────────────────────────────────────────────

class AssetPackager:
    """High-level asset packer: scans a directory and produces .canasset files.

    Args:
        compress_textures: Whether to run texture compression
        generate_lods:     Whether to generate LoD levels
        thread_count:      Thread pool size for parallel processing
        verbose:           Verbose logging
    """

    def __init__(
        self,
        compress_textures: bool = True,
        generate_lods: bool = True,
        thread_count: int = 4,
        verbose: bool = False,
    ):
        self.compress_textures = compress_textures
        self.generate_lods = generate_lods
        self.thread_count = thread_count
        self.verbose = verbose

    def pack(self, input_dir: str, output_path: str) -> dict:
        """Pack all assets in `input_dir` into a single .canasset.

        Scans for:
        - Meshes: *.obj, *.glb, *.gltf
        - Textures: *.png, *.jpg, *.tga, *.tiff
        - Materials: *.json (material definitions)

        Processing order:
        1. Discover all source files
        2. Generate LoDs for all meshes (parallel)
        3. Compress all textures (parallel)
        4. Pack into single .canasset

        Returns stats dict for CLI display.
        """
        from canopy_pipeline.lod_generator import LodGenerator
        from canopy_pipeline.texture_compress import TextureCompressor

        input_dir_path = Path(input_dir)
        meshes = list(input_dir_path.glob("**/*.obj"))
        textures = list(input_dir_path.glob("**/*.png")) + list(input_dir_path.glob("**/*.jpg"))
        total_assets = len(meshes) + len(textures)

        log.info("Discovered %d meshes, %d textures", len(meshes), len(textures))

        writer = CanAssetWriter()

        # Add metadata
        writer.add_metadata({
            "source_dir": str(input_dir_path.resolve()),
            "mesh_count": len(meshes),
            "texture_count": len(textures),
            "pipeline_version": "0.1.0",
        })

        # Process meshes
        if self.generate_lods:
            lod_gen = LodGenerator(verbose=self.verbose)
            for i, mesh_path in enumerate(meshes):
                log.info("[%d/%d] Processing mesh: %s", i + 1, len(meshes), mesh_path.name)
                try:
                    mesh = lod_gen._load_mesh(str(mesh_path))
                    lod_result = lod_gen._generate_lods(mesh, str(mesh_path))

                    lod_section_kinds = [
                        SectionKind.MESH_LOD0,
                        SectionKind.MESH_LOD1,
                        SectionKind.MESH_LOD2,
                        SectionKind.MESH_LOD3,
                    ]
                    for lod_level, section_kind in zip(lod_result.lods, lod_section_kinds):
                        mesh_bytes = encode_mesh(lod_level.mesh)
                        writer.add_section(Section(section_kind, mesh_bytes))

                    if lod_result.shadow:
                        shadow_bytes = encode_mesh(lod_result.shadow.mesh)
                        writer.add_section(Section(SectionKind.MESH_SHADOW, shadow_bytes))

                except Exception as e:
                    log.error("Failed to process mesh %s: %s", mesh_path, e)

        # Process textures
        if self.compress_textures:
            tex_section_map = {
                "albedo": SectionKind.TEXTURE_ALBEDO,
                "normal": SectionKind.TEXTURE_NORMAL,
                "roughness": SectionKind.TEXTURE_ROUGHNESS_METAL,
                "emissive": SectionKind.TEXTURE_EMISSIVE,
                "ao": SectionKind.TEXTURE_AO,
            }
            compressor = TextureCompressor(format="raw", generate_mipmaps=True, verbose=self.verbose)

            for tex_path in textures:
                stem = tex_path.stem.lower()
                # Detect texture type from filename convention: *_albedo.png, *_normal.png etc.
                section_kind = SectionKind.TEXTURE_ALBEDO  # default
                for keyword, kind in tex_section_map.items():
                    if keyword in stem:
                        section_kind = kind
                        break

                try:
                    import tempfile, os
                    with tempfile.NamedTemporaryFile(suffix=".texblob", delete=False) as tmp:
                        tmp_path = tmp.name
                    compressor.compress(str(tex_path), tmp_path)
                    tex_data = Path(tmp_path).read_bytes()
                    os.unlink(tmp_path)
                    writer.add_section(Section(section_kind, tex_data))
                except Exception as e:
                    log.error("Failed to process texture %s: %s", tex_path, e)

        total_bytes = writer.write(output_path)

        return {
            "asset_count": total_assets,
            "total_size_mb": total_bytes / 1024 / 1024,
        }


def write_canasset_with_lods(lod_result, output_path: str) -> None:
    """Write a LodResult directly to a .canasset file.

    Used by the LoD generator's process() method.
    """
    writer = CanAssetWriter()
    writer.add_metadata({"source": lod_result.source_path, "pipeline": "lod_only"})

    lod_section_kinds = [
        SectionKind.MESH_LOD0,
        SectionKind.MESH_LOD1,
        SectionKind.MESH_LOD2,
        SectionKind.MESH_LOD3,
    ]
    for lod_level, section_kind in zip(lod_result.lods, lod_section_kinds):
        writer.add_section(Section(section_kind, encode_mesh(lod_level.mesh)))

    if lod_result.shadow:
        writer.add_section(Section(SectionKind.MESH_SHADOW, encode_mesh(lod_result.shadow.mesh)))

    writer.write(output_path)
