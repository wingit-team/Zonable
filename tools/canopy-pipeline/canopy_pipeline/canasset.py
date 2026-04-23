"""CanAsset binary reader — for the `info` CLI command and testing."""

from __future__ import annotations

import struct
from dataclasses import dataclass
from pathlib import Path
from typing import Iterator

from canopy_pipeline.asset_packager import (
    CANASSET_MAGIC, CANASSET_VERSION, HEADER_SIZE, SECTION_ENTRY_SIZE, SectionKind
)


@dataclass
class SectionInfo:
    index: int
    kind: int
    offset: int
    size: int
    flags: int

    @property
    def kind_name(self) -> str:
        return SectionKind.name(self.kind)


class CanAssetReader:
    """Read and inspect a .canasset file."""

    def __init__(self, path: str):
        self.path = Path(path)
        self.data = self.path.read_bytes()
        self.file_size = len(self.data)
        self._parse_header()

    def _parse_header(self) -> None:
        if len(self.data) < HEADER_SIZE:
            raise ValueError(f"File too small ({self.file_size} bytes) to be a .canasset")

        magic, version, section_count, total_size, uuid_bytes, reserved = struct.unpack_from(
            "<8sIIQ16s24s", self.data, 0
        )

        if magic != CANASSET_MAGIC:
            raise ValueError(f"Invalid magic: {magic!r}")
        if version != CANASSET_VERSION:
            raise ValueError(f"Version mismatch: {version} != {CANASSET_VERSION}")

        self.version = version
        self.section_count = section_count
        self.total_size = total_size
        self.uuid_hex = uuid_bytes.hex()

        # Parse section table
        self.sections: list[SectionInfo] = []
        for i in range(section_count):
            offset_in_file = HEADER_SIZE + i * SECTION_ENTRY_SIZE
            kind, data_offset, size, flags = struct.unpack_from(
                "<IIII", self.data, offset_in_file
            )
            self.sections.append(SectionInfo(
                index=i, kind=kind, offset=data_offset, size=size, flags=flags
            ))

    def get_section_data(self, kind: int) -> bytes | None:
        for section in self.sections:
            if section.kind == kind:
                start = section.offset
                return self.data[start:start + section.size]
        return None
