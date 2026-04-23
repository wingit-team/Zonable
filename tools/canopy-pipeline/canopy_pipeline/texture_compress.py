"""Texture compressor — BC7/ASTC/BC5 block compression.

# Compression Formats

| Format | Use Case                    | GPU Support                    |
|--------|-----------------------------|--------------------------------|
| BC7    | Albedo, any 4-channel       | All desktop: DX11+, Vulkan, Metal 2+ |
| BC5    | Normal maps (RG only)       | All desktop (DX10+)           |
| ASTC   | Mobile, Apple Silicon       | iOS, Android (Adreno/Mali), Apple M-series |
| RAW    | Development / HDR textures  | Always                        |

# Phase 1 vs Phase 2

Phase 1: Uses Pillow for basic image loading and mipmap generation.
         No actual GPU block compression — outputs raw RGBA8 + metadata.
         This is sufficient for development iteration without the compression latency.

Phase 2: Integrates one of:
  - `compressonator-sdk` (AMD's open-source BC7/ASTC compressor)
  - `bc7enc` via ctypes (fast BC7 encoder)
  - `astc-encoder` subprocess (ARM's reference ASTC encoder)
  - Use platform GPU APIs (Metal's MTLTexture.makeTextureView etc.) for lossless

# Mipmap Algorithm

Mipmaps are generated with Lanczos resampling (Pillow's LANCZOS filter).
For normal maps (BC5), each mip level requires re-normalization:
  1. Decode RG → reconstruct Z
  2. Downsample
  3. Renormalize
  4. Re-encode to RG

This prevents normal map blurring at distance (a common bug when using
box-filter mipmaps on normal maps).
"""

from __future__ import annotations

import io
import logging
import math
import struct
from dataclasses import dataclass
from pathlib import Path

from PIL import Image
import numpy as np

log = logging.getLogger(__name__)

# .canasset format constants (must match Rust format.rs)
SECTION_KIND_TEXTURE_ALBEDO = 7
SECTION_KIND_TEXTURE_NORMAL = 8
SECTION_KIND_TEXTURE_ROUGHNESS_METAL = 9
SECTION_KIND_TEXTURE_EMISSIVE = 10
SECTION_KIND_TEXTURE_AO = 11

FORMAT_CODES = {
    "raw": 0,
    "bc7": 1,
    "astc": 2,
    "bc5": 3,
}


@dataclass
class TextureResult:
    format: str
    width: int
    height: int
    mip_count: int
    original_size_kb: float
    compressed_size_kb: float

    @property
    def ratio(self) -> float:
        return self.original_size_kb / max(self.compressed_size_kb, 0.001)


class TextureCompressor:
    """Compress source textures to BC7/ASTC/BC5 with mipmap generation.

    Args:
        format:           Output format ('bc7', 'astc', 'bc5', 'raw')
        quality:          Compression quality level ('fast', 'medium', 'high', 'ultra')
        generate_mipmaps: Whether to generate the full mip chain
        max_size:         Maximum texture dimension — image is downsampled if larger
        verbose:          Print per-mip progress
    """

    def __init__(
        self,
        format: str = "bc7",
        quality: str = "high",
        generate_mipmaps: bool = True,
        max_size: int = 4096,
        verbose: bool = False,
    ):
        self.format = format
        self.quality = quality
        self.generate_mipmaps = generate_mipmaps
        self.max_size = max_size
        self.verbose = verbose

    def compress(self, input_path: str, output_path: str) -> TextureResult:
        """Load, compress, and write texture data.

        Returns metadata for CLI display.
        """
        img = Image.open(input_path)
        original_bytes = Path(input_path).stat().st_size

        # Ensure power-of-two dimensions (required for full mip chain)
        img = self._resize_to_pow2(img)

        # Enforce max size
        if img.width > self.max_size or img.height > self.max_size:
            img = img.resize(
                (min(img.width, self.max_size), min(img.height, self.max_size)),
                Image.LANCZOS,
            )
            log.info("Resized to %dx%d (max_size=%d)", img.width, img.height, self.max_size)

        # Convert to appropriate mode
        if self.format == "bc5":
            # Normal map — only need RG channels
            img = img.convert("RGB")
        else:
            img = img.convert("RGBA")

        # Generate mipmaps
        mips = self._generate_mips(img)

        # Compress each mip
        compressed_mips: list[bytes] = []
        for i, mip in enumerate(mips):
            data = self._compress_mip(mip, level=i)
            compressed_mips.append(data)
            if self.verbose:
                log.debug("  Mip %d: %dx%d → %d bytes", i, mip.width, mip.height, len(data))

        # Pack into a simple texture blob (format defined in canasset.py)
        packed = self._pack_texture_blob(compressed_mips, img.width, img.height)

        Path(output_path).write_bytes(packed)

        return TextureResult(
            format=self.format,
            width=img.width,
            height=img.height,
            mip_count=len(mips),
            original_size_kb=original_bytes / 1024,
            compressed_size_kb=len(packed) / 1024,
        )

    def _resize_to_pow2(self, img: Image.Image) -> Image.Image:
        """Resize image to next power-of-two dimensions."""
        def next_pow2(n: int) -> int:
            return 1 << math.ceil(math.log2(max(n, 1)))

        new_w = next_pow2(img.width)
        new_h = next_pow2(img.height)

        if new_w != img.width or new_h != img.height:
            log.info("Resizing %dx%d → %dx%d (pow2)", img.width, img.height, new_w, new_h)
            img = img.resize((new_w, new_h), Image.LANCZOS)
        return img

    def _generate_mips(self, img: Image.Image) -> list[Image.Image]:
        """Generate full mipmap chain from mip0 down to 1x1."""
        mips = [img]
        w, h = img.width // 2, img.height // 2

        while w >= 1 and h >= 1:
            if self.format == "bc5":
                # Normal map mip: renormalize RG after downsampling
                mip = self._downsample_normal_map(mips[-1])
            else:
                mip = mips[-1].resize((w, h), Image.LANCZOS)
            mips.append(mip)
            w //= 2
            h //= 2

        return mips

    def _downsample_normal_map(self, img: Image.Image) -> Image.Image:
        """Downsample a normal map with proper renormalization.

        1. Decode RG → reconstruct Z (assume Z = sqrt(1 - R² - G²))
        2. Downsample all 3 channels with Lanczos
        3. Renormalize the XYZ vector
        4. Re-encode to RG
        """
        arr = np.array(img, dtype=np.float32) / 255.0
        r, g = arr[:, :, 0], arr[:, :, 1]
        # Remap from [0,1] to [-1,1]
        nx = r * 2.0 - 1.0
        ny = g * 2.0 - 1.0
        nz_sq = np.clip(1.0 - nx**2 - ny**2, 0.0, 1.0)
        nz = np.sqrt(nz_sq)

        # Downsample each channel
        half_w, half_h = img.width // 2, img.height // 2
        def downsample_channel(ch: np.ndarray) -> np.ndarray:
            pil_ch = Image.fromarray(((ch * 0.5 + 0.5) * 255).astype(np.uint8), mode="L")
            pil_ch = pil_ch.resize((half_w, half_h), Image.LANCZOS)
            return np.array(pil_ch, dtype=np.float32) / 255.0 * 2.0 - 1.0

        nx2, ny2, nz2 = downsample_channel(nx), downsample_channel(ny), downsample_channel(nz)

        # Renormalize
        length = np.sqrt(np.maximum(nx2**2 + ny2**2 + nz2**2, 1e-8))
        nx2, ny2 = nx2 / length, ny2 / length

        # Re-encode to uint8 RG
        r_out = ((nx2 * 0.5 + 0.5) * 255).clip(0, 255).astype(np.uint8)
        g_out = ((ny2 * 0.5 + 0.5) * 255).clip(0, 255).astype(np.uint8)
        b_out = np.zeros_like(r_out)  # Unused but needed for RGB image

        return Image.fromarray(np.stack([r_out, g_out, b_out], axis=2), mode="RGB")

    def _compress_mip(self, img: Image.Image, level: int = 0) -> bytes:
        """Compress a single mip level.

        Phase 1: Returns raw RGBA/RGB bytes (no GPU compression).
        Phase 2: Call bc7enc or compressonator SDK for actual block compression.

        Block compression reduces texture memory by 4-8x (BC7: 8 bpp vs 32 bpp raw).
        This is critical for a city builder with thousands of unique building textures.
        """
        if self.format == "raw":
            return img.tobytes()
        elif self.format in ("bc7", "astc"):
            # Phase 1: just use raw RGBA bytes as placeholder
            # Phase 2: integrate bc7enc or compressonator
            log.debug("format=%s: Phase 1 stub — using raw bytes (Phase 2 will compress)", self.format)
            return img.convert("RGBA").tobytes()
        elif self.format == "bc5":
            # Phase 1: raw RG bytes
            arr = np.array(img)[:, :, :2]  # RG only
            return arr.tobytes()
        else:
            raise ValueError(f"Unknown format: {self.format}")

    def _pack_texture_blob(
        self,
        mips: list[bytes],
        base_width: int,
        base_height: int,
    ) -> bytes:
        """Pack compressed mip data into a texture blob.

        Layout:
            [4B] format_code
            [4B] width
            [4B] height
            [4B] mip_count
            [4B × mip_count] mip_offsets (from start of mip data section)
            [N bytes] mip data (mip0, mip1, ..., mipN)
        """
        mip_count = len(mips)
        header_size = 4 + 4 + 4 + 4 + (4 * mip_count)

        offsets = []
        offset = 0
        for mip in mips:
            offsets.append(offset)
            offset += len(mip)

        header = struct.pack(
            f"<IIII{mip_count}I",
            FORMAT_CODES.get(self.format, 0),
            base_width,
            base_height,
            mip_count,
            *offsets,
        )

        return header + b"".join(mips)
