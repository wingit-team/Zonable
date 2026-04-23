"""LoD generator — meshoptimizer-based polygon reduction.

Generates multiple Levels of Detail from a source mesh using the
meshoptimizer simplification algorithm. For each LoD level we target
a vertex count ratio (0.5 = half the vertices of the previous level)
with a maximum screen-space error threshold.

# Algorithm

meshoptimizer's `simplify` function uses a quadric error metric (QEM)
similar to the Garland-Heckbert algorithm but with modifications for:
- Attribute seam preservation (UV seams, normal discontinuities)
- Topology preservation (no handles/holes introduced)
- Lock border vertices (prevent simplifying mesh boundaries)

For building meshes in Zonable specifically, we also want:
- Preserve silhouette edges (important for city skyline at distance)
- Preserve UV island boundaries (texture seams must remain intact)

# Phase 2 — meshoptimizer PyPI bindings

This module currently uses a pure-Python approximation (vertex clustering)
as a placeholder. Phase 2 will use the `meshoptimizer` PyPI package
(https://pypi.org/project/meshoptimizer/) which wraps the official C library.

Replace `_simplify_vertex_cluster` with:
    import meshoptimizer
    indices_out, error = meshoptimizer.simplify(
        indices, vertices, target_count, target_error, options
    )
"""

from __future__ import annotations

import logging
import math
import struct
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import numpy as np

log = logging.getLogger(__name__)


@dataclass
class MeshData:
    """Raw mesh data in a pipeline-friendly format."""
    vertices: np.ndarray  # shape (N, 3) float32 — positions
    normals: np.ndarray   # shape (N, 3) float32
    uvs: np.ndarray       # shape (N, 2) float32
    indices: np.ndarray   # shape (M,) uint32 — triangle indices (M must be divisible by 3)

    @property
    def vertex_count(self) -> int:
        return len(self.vertices)

    @property
    def triangle_count(self) -> int:
        return len(self.indices) // 3

    def memory_bytes(self) -> int:
        return (self.vertices.nbytes + self.normals.nbytes +
                self.uvs.nbytes + self.indices.nbytes)


@dataclass
class LodLevel:
    name: str
    mesh: MeshData
    ratio: float
    target_ratio: float
    size_bytes: int


@dataclass
class LodResult:
    source_path: str
    lods: list[LodLevel]
    shadow: LodLevel | None

    def as_stats_dict(self) -> dict:
        return {
            "lods": [
                {
                    "name": l.name,
                    "vertices": l.mesh.vertex_count,
                    "triangles": l.mesh.triangle_count,
                    "size_bytes": l.size_bytes,
                    "ratio": l.ratio,
                }
                for l in self.lods
            ]
        }


class LodGenerator:
    """Generates LoD meshes from a source mesh.

    Args:
        lod_ratios:        Vertex count ratios per LoD level (1.0 = original).
                           E.g. [1.0, 0.5, 0.25, 0.1] for 4 levels.
        shadow_ratio:      Vertex ratio for the shadow proxy mesh (very low poly).
        error_threshold:   Maximum allowed QEM error per simplification step.
                           0.01 = 1% of bounding box diagonal.
        verbose:           Print progress per LoD.
    """

    def __init__(
        self,
        lod_ratios: list[float] | None = None,
        shadow_ratio: float = 0.05,
        error_threshold: float = 0.01,
        verbose: bool = False,
    ):
        self.lod_ratios = lod_ratios or [1.0, 0.5, 0.25, 0.1]
        self.shadow_ratio = shadow_ratio
        self.error_threshold = error_threshold
        self.verbose = verbose

    def process(self, input_path: str, output_path: str) -> dict:
        """Full pipeline: load → generate LoDs → write .canasset.

        Returns a stats dict suitable for CLI display.
        """
        log.info("Loading mesh: %s", input_path)
        mesh = self._load_mesh(input_path)
        log.info("Loaded %d vertices, %d triangles", mesh.vertex_count, mesh.triangle_count)

        result = self._generate_lods(mesh, source_path=input_path)

        from canopy_pipeline.asset_packager import write_canasset_with_lods
        write_canasset_with_lods(result, output_path)

        return result.as_stats_dict()

    def _load_mesh(self, path: str) -> MeshData:
        """Load a mesh from .obj / .glb / .gltf.

        Phase 1: Only .obj is supported (pure Python, no C dependencies).
        Phase 2: Use `trimesh` or `pygltflib` for .glb/.gltf/.fbx.
        """
        suffix = Path(path).suffix.lower()
        if suffix == ".obj":
            return self._load_obj(path)
        else:
            raise NotImplementedError(
                f"Mesh format '{suffix}' not yet supported. "
                f"Supported: .obj. Phase 2 will add .glb, .gltf, .fbx via trimesh."
            )

    def _load_obj(self, path: str) -> MeshData:
        """Minimal Wavefront OBJ loader.

        Handles: v, vn, vt, f (triangles only).
        Does NOT handle: materials, multi-mesh, quads (Phase 2: use trimesh).
        """
        positions: list[tuple[float, float, float]] = []
        normals_src: list[tuple[float, float, float]] = []
        uvs_src: list[tuple[float, float]] = []

        # Final interleaved vertex data (de-indexed)
        vert_positions: list[tuple[float, float, float]] = []
        vert_normals: list[tuple[float, float, float]] = []
        vert_uvs: list[tuple[float, float]] = []
        indices: list[int] = []

        # Cache of (pos_idx, uv_idx, norm_idx) → final vertex index
        vert_cache: dict[tuple[int, int, int], int] = {}

        with open(path) as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                parts = line.split()
                if parts[0] == "v":
                    positions.append((float(parts[1]), float(parts[2]), float(parts[3])))
                elif parts[0] == "vn":
                    normals_src.append((float(parts[1]), float(parts[2]), float(parts[3])))
                elif parts[0] == "vt":
                    uvs_src.append((float(parts[1]), float(parts[2])))
                elif parts[0] == "f":
                    face_verts = parts[1:]
                    # Triangulate fans for quads/polygons
                    parsed = [self._parse_face_vert(fv) for fv in face_verts]
                    for i in range(1, len(parsed) - 1):
                        for v in [parsed[0], parsed[i], parsed[i + 1]]:
                            key = v
                            if key not in vert_cache:
                                vert_cache[key] = len(vert_positions)
                                vert_positions.append(positions[v[0]] if v[0] < len(positions) else (0.0, 0.0, 0.0))
                                vert_normals.append(normals_src[v[2]] if v[2] >= 0 and v[2] < len(normals_src) else (0.0, 1.0, 0.0))
                                vert_uvs.append(uvs_src[v[1]] if v[1] >= 0 and v[1] < len(uvs_src) else (0.0, 0.0))
                            indices.append(vert_cache[key])

        return MeshData(
            vertices=np.array(vert_positions, dtype=np.float32),
            normals=np.array(vert_normals, dtype=np.float32),
            uvs=np.array(vert_uvs, dtype=np.float32),
            indices=np.array(indices, dtype=np.uint32),
        )

    def _parse_face_vert(self, s: str) -> tuple[int, int, int]:
        """Parse OBJ face vertex 'v/vt/vn' → (pos_idx, uv_idx, norm_idx) (0-based)."""
        parts = s.split("/")
        pos_idx = int(parts[0]) - 1
        uv_idx = int(parts[1]) - 1 if len(parts) > 1 and parts[1] else -1
        norm_idx = int(parts[2]) - 1 if len(parts) > 2 and parts[2] else -1
        return (pos_idx, uv_idx, norm_idx)

    def _generate_lods(self, source: MeshData, source_path: str) -> LodResult:
        lod_levels: list[LodLevel] = []

        for i, ratio in enumerate(self.lod_ratios):
            name = f"lod{i}"
            if self.verbose:
                log.info("Generating %s (ratio=%.2f)...", name, ratio)

            simplified = self._simplify(source, ratio)
            lod_levels.append(LodLevel(
                name=name,
                mesh=simplified,
                ratio=simplified.vertex_count / source.vertex_count,
                target_ratio=ratio,
                size_bytes=simplified.memory_bytes(),
            ))

        shadow_mesh = self._simplify(source, self.shadow_ratio)
        shadow = LodLevel(
            name="shadow",
            mesh=shadow_mesh,
            ratio=shadow_mesh.vertex_count / source.vertex_count,
            target_ratio=self.shadow_ratio,
            size_bytes=shadow_mesh.memory_bytes(),
        )

        return LodResult(source_path=source_path, lods=lod_levels, shadow=shadow)

    def _simplify(self, mesh: MeshData, target_ratio: float) -> MeshData:
        """Simplify a mesh to approximately `target_ratio` of its original vertex count.

        Phase 1: Uses vertex clustering (fast but lower quality).
        Phase 2: Replace with meshoptimizer.simplify() for QEM-based reduction.

        Vertex clustering: divides the bounding box into a grid, merges vertices
        within the same cell. Simple, fast, but doesn't preserve attributes well.
        QEM is much better for preserving surface detail.
        """
        if target_ratio >= 0.99:
            return mesh  # LoD0 — unchanged

        target_vertices = max(3, int(mesh.vertex_count * target_ratio))
        return self._simplify_vertex_cluster(mesh, target_vertices)

    def _simplify_vertex_cluster(self, mesh: MeshData, target_count: int) -> MeshData:
        """Vertex clustering simplification.

        Divides bounding box into a grid. All vertices in the same cell
        are merged to their centroid. Triangles where all 3 corners merge
        to the same cell are degenerate and are removed.

        Grid resolution is chosen so that total cells ≈ target_count.
        """
        verts = mesh.vertices
        mn, mx = verts.min(axis=0), verts.max(axis=0)
        span = mx - mn + 1e-6

        # Choose grid resolution: cbrt(target_count) per axis
        grid_res = max(2, int(math.ceil(target_count ** (1/3))))
        cell_size = span / grid_res

        # Map each vertex to a grid cell
        cell_idx = ((verts - mn) / cell_size).astype(np.int32)
        cell_idx = np.clip(cell_idx, 0, grid_res - 1)
        cell_keys = (cell_idx[:, 0] * grid_res * grid_res +
                     cell_idx[:, 1] * grid_res +
                     cell_idx[:, 2])

        # Build representative vertex for each cell (mean position)
        unique_cells, inverse = np.unique(cell_keys, return_inverse=True)
        n_cells = len(unique_cells)

        new_verts = np.zeros((n_cells, 3), dtype=np.float32)
        new_norms = np.zeros((n_cells, 3), dtype=np.float32)
        new_uvs = np.zeros((n_cells, 2), dtype=np.float32)
        counts = np.zeros(n_cells, dtype=np.int32)

        np.add.at(new_verts, inverse, verts)
        np.add.at(new_norms, inverse, mesh.normals)
        np.add.at(new_uvs, inverse, mesh.uvs)
        np.add.at(counts, inverse, 1)

        counts_safe = np.maximum(counts, 1)[:, None]
        new_verts /= counts_safe
        new_norms /= counts_safe
        new_uvs /= counts_safe[:, :1] * np.ones((1, 2))

        # Normalize normals
        norm_len = np.linalg.norm(new_norms, axis=1, keepdims=True)
        new_norms /= np.maximum(norm_len, 1e-6)

        # Remap indices
        new_indices = inverse[mesh.indices]
        triangles = new_indices.reshape(-1, 3)

        # Remove degenerate triangles (all 3 vertices in same cell)
        valid = ~((triangles[:, 0] == triangles[:, 1]) |
                  (triangles[:, 1] == triangles[:, 2]) |
                  (triangles[:, 0] == triangles[:, 2]))
        triangles = triangles[valid]

        if len(triangles) == 0:
            # Fallback: return a minimal mesh if everything was collapsed
            triangles = np.array([[0, 1, 2]], dtype=np.uint32)
            new_verts = mesh.vertices[:3]
            new_norms = mesh.normals[:3]
            new_uvs = mesh.uvs[:3]

        return MeshData(
            vertices=new_verts.astype(np.float32),
            normals=new_norms.astype(np.float32),
            uvs=new_uvs.astype(np.float32),
            indices=triangles.flatten().astype(np.uint32),
        )
