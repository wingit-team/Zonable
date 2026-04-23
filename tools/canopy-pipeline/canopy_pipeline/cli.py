"""Canopy Pipeline — main CLI entry point.

Usage:
    canopy-pipeline lod      <input> [--output OUTPUT] [--lod-levels 4]
    canopy-pipeline compress <input> [--format bc7|astc] [--quality high]
    canopy-pipeline pack     <dir>   [--output OUTPUT.canasset]
    canopy-pipeline bake     <world> [--threads N]
    canopy-pipeline info     <file>  — dump .canasset header info
"""

import click
from rich.console import Console
from rich.table import Table
import sys

console = Console()


@click.group()
@click.version_option("0.1.0", prog_name="canopy-pipeline")
@click.option("--verbose", "-v", is_flag=True, help="Enable verbose logging")
@click.pass_context
def main(ctx: click.Context, verbose: bool) -> None:
    """Canopy Engine asset pipeline tools."""
    ctx.ensure_object(dict)
    ctx.obj["verbose"] = verbose


# ---------------------------------------------------------------------------
# LOD generation
# ---------------------------------------------------------------------------

@main.command()
@click.argument("input_path", metavar="INPUT", type=click.Path(exists=True))
@click.option("--output", "-o", type=click.Path(), default=None,
              help="Output .canasset path (default: replaces extension)")
@click.option("--lod-levels", default=4, show_default=True,
              help="Number of LoD levels to generate (1-4)")
@click.option("--ratios", default="1.0,0.5,0.25,0.1",
              help="Comma-separated vertex reduction ratios per LoD level")
@click.option("--shadow-ratio", default=0.05, show_default=True,
              help="Vertex ratio for shadow proxy mesh")
@click.option("--error-threshold", default=0.01, show_default=True,
              help="Meshoptimizer error threshold (0.0-1.0)")
@click.pass_context
def lod(ctx: click.Context, input_path: str, output: str | None,
        lod_levels: int, ratios: str, shadow_ratio: float,
        error_threshold: float) -> None:
    """Generate LoD meshes from a source mesh file.

    Accepts: .obj, .glb, .gltf, .fbx (via trimesh)
    Produces: .canasset with embedded LoD0-3 + shadow mesh sections.

    The LoD generation uses meshoptimizer's simplify algorithm:
    - LoD0: 100% (original)
    - LoD1: ~50% vertex reduction
    - LoD2: ~25% vertex reduction
    - LoD3: ~10% vertex reduction
    - Shadow: ~5% vertex reduction (shadow proxy)

    Screen-space error thresholds ensure visual quality at each level.
    """
    from canopy_pipeline.lod_generator import LodGenerator

    ratio_list = [float(r.strip()) for r in ratios.split(",")][:lod_levels]
    output_path = output or str(input_path).rsplit(".", 1)[0] + ".canasset"

    console.print(f"[bold cyan]Canopy Pipeline — LoD Generation[/bold cyan]")
    console.print(f"  Input:      {input_path}")
    console.print(f"  Output:     {output_path}")
    console.print(f"  LoD levels: {lod_levels} (ratios: {ratio_list})")

    generator = LodGenerator(
        lod_ratios=ratio_list,
        shadow_ratio=shadow_ratio,
        error_threshold=error_threshold,
        verbose=ctx.obj["verbose"],
    )

    try:
        result = generator.process(input_path, output_path)
        console.print(f"[green]✓ LoD generation complete[/green]")
        _print_lod_stats(result)
    except Exception as e:
        console.print(f"[red]✗ LoD generation failed: {e}[/red]")
        sys.exit(1)


def _print_lod_stats(result: dict) -> None:
    table = Table(title="LoD Statistics")
    table.add_column("Level", style="cyan")
    table.add_column("Vertices", justify="right")
    table.add_column("Triangles", justify="right")
    table.add_column("Size (KB)", justify="right")
    table.add_column("Ratio", justify="right")

    for entry in result.get("lods", []):
        table.add_row(
            entry["name"],
            str(entry["vertices"]),
            str(entry["triangles"]),
            f"{entry['size_bytes'] / 1024:.1f}",
            f"{entry['ratio']:.2f}",
        )
    console.print(table)


# ---------------------------------------------------------------------------
# Texture compression
# ---------------------------------------------------------------------------

@main.command()
@click.argument("input_path", metavar="INPUT", type=click.Path(exists=True))
@click.option("--output", "-o", type=click.Path(), default=None)
@click.option("--format", "fmt", type=click.Choice(["bc7", "astc", "bc5", "raw"]),
              default="bc7", show_default=True, help="Output compression format")
@click.option("--quality", type=click.Choice(["fast", "medium", "high", "ultra"]),
              default="high", show_default=True)
@click.option("--generate-mipmaps/--no-mipmaps", default=True, show_default=True)
@click.option("--max-size", default=4096, show_default=True,
              help="Maximum texture dimension (downsampled if exceeded)")
@click.pass_context
def compress(ctx: click.Context, input_path: str, output: str | None,
             fmt: str, quality: str, generate_mipmaps: bool, max_size: int) -> None:
    """Compress a texture to BC7/ASTC/BC5 format.

    BC7:  Desktop (Vulkan, DX12, Metal) — excellent quality, all channels
    ASTC: Mobile / Apple Silicon — flexible block size, good quality
    BC5:  Normal maps only (RG channels, reconstruct Z in shader)

    Mipmap generation uses a Lanczos filter. For normal maps, proper
    RG normalization is applied at each mip level to prevent artifacts.
    """
    from canopy_pipeline.texture_compress import TextureCompressor

    output_path = output or str(input_path).rsplit(".", 1)[0] + f".{fmt}.canasset"

    console.print(f"[bold cyan]Canopy Pipeline — Texture Compression[/bold cyan]")
    console.print(f"  Input:   {input_path}")
    console.print(f"  Format:  {fmt.upper()}")
    console.print(f"  Quality: {quality}")
    console.print(f"  Mipmaps: {generate_mipmaps}")

    compressor = TextureCompressor(
        format=fmt,
        quality=quality,
        generate_mipmaps=generate_mipmaps,
        max_size=max_size,
        verbose=ctx.obj["verbose"],
    )

    try:
        result = compressor.compress(input_path, output_path)
        console.print(f"[green]✓ Compressed {result['original_size_kb']:.0f} KB → "
                      f"{result['compressed_size_kb']:.0f} KB "
                      f"({result['ratio']:.1f}x)[/green]")
    except Exception as e:
        console.print(f"[red]✗ Compression failed: {e}[/red]")
        sys.exit(1)


# ---------------------------------------------------------------------------
# Asset packager
# ---------------------------------------------------------------------------

@main.command()
@click.argument("input_dir", metavar="DIR", type=click.Path(exists=True, file_okay=False))
@click.option("--output", "-o", type=click.Path(), required=True)
@click.option("--no-compress-textures", is_flag=True, default=False,
              help="Skip texture compression (faster, larger output)")
@click.option("--generate-lods/--no-lods", default=True)
@click.option("--threads", default=4, show_default=True)
@click.pass_context
def pack(ctx: click.Context, input_dir: str, output: str,
         no_compress_textures: bool, generate_lods: bool, threads: int) -> None:
    """Pack a directory of source assets into a single .canasset file.

    The packager:
    1. Scans the directory for meshes (.obj, .glb, .fbx)
    2. Generates LoD levels for each mesh (unless --no-lods)
    3. Compresses textures to BC7/ASTC (unless --no-compress-textures)
    4. Writes a single .canasset file with all data packed in section order:
       header → section table → mesh sections → texture sections → material section

    The output file is memory-mappable — the section table gives direct byte
    offsets to each data block, enabling zero-copy GPU upload at runtime.
    """
    from canopy_pipeline.asset_packager import AssetPackager

    console.print(f"[bold cyan]Canopy Pipeline — Asset Packager[/bold cyan]")
    console.print(f"  Input dir:  {input_dir}")
    console.print(f"  Output:     {output}")
    console.print(f"  LoD gen:    {generate_lods}")
    console.print(f"  Tex compress: {not no_compress_textures}")

    packager = AssetPackager(
        compress_textures=not no_compress_textures,
        generate_lods=generate_lods,
        thread_count=threads,
        verbose=ctx.obj["verbose"],
    )

    try:
        result = packager.pack(input_dir, output)
        console.print(f"[green]✓ Packed {result['asset_count']} assets → {output} "
                      f"({result['total_size_mb']:.1f} MB)[/green]")
    except Exception as e:
        console.print(f"[red]✗ Pack failed: {e}[/red]")
        sys.exit(1)


# ---------------------------------------------------------------------------
# Asset info dump
# ---------------------------------------------------------------------------

@main.command()
@click.argument("input_path", metavar="FILE", type=click.Path(exists=True))
def info(input_path: str) -> None:
    """Print header and section info for a .canasset file."""
    from canopy_pipeline.canasset import CanAssetReader

    try:
        reader = CanAssetReader(input_path)
        console.print(f"\n[bold]File:[/bold] {input_path}")
        console.print(f"[bold]Size:[/bold] {reader.file_size / 1024 / 1024:.2f} MB")
        console.print(f"[bold]UUID:[/bold] {reader.uuid_hex}")
        console.print(f"[bold]Version:[/bold] {reader.version}")

        table = Table(title="Sections")
        table.add_column("Index", justify="right")
        table.add_column("Kind")
        table.add_column("Offset", justify="right")
        table.add_column("Size (KB)", justify="right")

        for i, section in enumerate(reader.sections):
            table.add_row(
                str(i),
                section.kind_name,
                hex(section.offset),
                f"{section.size / 1024:.1f}",
            )
        console.print(table)
    except Exception as e:
        console.print(f"[red]✗ Failed to read {input_path}: {e}[/red]")
        sys.exit(1)


if __name__ == "__main__":
    main()
