# Canopy Engine

A custom game engine written in **Rust** with **Python** as the primary scripting layer (via [PyO3](https://pyo3.rs)).

Architecture is inspired by [Panda3D](https://www.panda3d.org/) — the engine core is Rust, but game developers interact entirely through Python. Heavy game systems can optionally be written in Rust, but the default game code path is Python.

## Games Powered By Canopy

| Game | Status | Description |
|------|--------|-------------|
| **Zonable** | 🚧 In Development | City builder with realistic economic causality, hybrid agent/statistical citizen simulation, flow-based traffic, procedural world and building generation |
| **Untitled RPG** | 📅 2029–2031 | Large-scale procedural open world RPG |

## Engine Architecture

```
canopy-engine/
├── crates/
│   ├── canopy-core        # App loop, engine lifecycle, plugin system
│   ├── canopy-renderer    # wgpu renderer — Vulkan/Metal/DX12/WebGPU
│   ├── canopy-ecs         # Custom ECS — hundreds of thousands of entities
│   ├── canopy-sim         # Simulation — agents, traffic flow, economics
│   ├── canopy-world       # Terrain, chunk streaming, procedural generation
│   ├── canopy-assets      # Asset runtime + .canasset proprietary format
│   ├── canopy-audio       # Audio system (cpal)
│   ├── canopy-physics     # Physics (Rapier 3D)
│   ├── canopy-script      # PyO3 bindings — the Python game API
│   └── canopy-platform    # Window creation, input (winit)
├── tools/
│   └── canopy-pipeline    # Python CLI: LoD generation, texture compression, asset packing
├── games/
│   └── zonable            # City builder game scripts
└── docs/                  # [Architecture](docs/architecture.md), [Renderer](docs/renderer.md), [Python API](docs/canscript-api.md), [Asset Format](docs/canasset-format.md)
```

## Crate Dependency Graph

```
canopy-platform  (winit, no engine deps)
     ↓
canopy-ecs       (no engine deps — pure ECS)
     ↓
canopy-assets    (depends on ecs for asset handles)
     ↓
canopy-audio     (depends on assets)
canopy-physics   (depends on ecs)
canopy-renderer  (depends on ecs, assets)
canopy-sim       (depends on ecs, world)
canopy-world     (depends on ecs, assets)
     ↓
canopy-core      (depends on all — orchestrates app loop)
     ↓
canopy-script    (depends on canopy-core — PyO3 bindings)
```

## Python API (canscript)

Game code is pure Python. Example:

```python
# Spawn a building
from canopy import world, Vec3, Quat
from canopy.components import Transform, Mesh, BuildingData

entity = world.spawn()
world.add(entity, Transform(position=Vec3(100, 0, 200)))
world.add(entity, Mesh(asset="assets/buildings/residential_01.canasset"))
world.add(entity, BuildingData(zone_id=42, capacity=48))
```

```python
# Define a simulation system
from canopy import System, Query
from canopy.components import BuildingData, Zone

class PopulationGrowthSystem(System):
    tick_rate = 4  # Hz

    def on_tick(self, dt: float, query: Query):
        for entity, (building, zone) in query.with_components(BuildingData, Zone):
            ...
```

```python
# Register disaster event handlers
from canopy import on_event
from canopy.sim import EarthquakeEvent

@on_event(EarthquakeEvent)
def handle_earthquake(event: EarthquakeEvent):
    ...
```

## Prerequisites

- Rust stable (see `rust-toolchain.toml`)
- Python ≥ 3.11
- [maturin](https://maturin.rs/) for building PyO3 extensions

## Building

```bash
# Check all crates
cargo check --workspace

# Build the Python extension (canopy module)
cd crates/canopy-script
maturin develop

# Run the asset pipeline
cd tools/canopy-pipeline
pip install -e .
canopy-pipeline --help
```

### macOS Build Notes
On Apple Silicon (M1/M2/M3), you must ensure that the linker uses `dynamic_lookup` for PyO3 symbols. This is handled automatically by the `.cargo/config.toml` provided in the repository:
```toml
[target.aarch64-apple-darwin]
rustflags = ["-C", "link-arg=-undefined", "-C", "link-arg=dynamic_lookup"]
```

## Key Design Decisions

- **ECS**: Custom dense SoA storage with archetype grouping. No dynamic dispatch in hot path.
- **Simulation budget**: Entities outside view run at 4Hz heartbeat; statistical pools used beyond active radius.
- **Traffic**: Fluid dynamics flow field on road network — not per-agent pathfinding.
- **Assets**: `.canasset` is a memory-mappable binary format — zero runtime parsing overhead.
- **Chunk streaming**: Predictive pre-loading based on entity velocity + look direction.
- **GIL strategy**: Python systems dispatch through a `ScriptRunner` that batches inputs before acquiring the GIL.

## F3 Performance Toolkit (Global)

The engine now ships with a built-in diagnostics toolkit that is always available in every game.

- `F3`: Toggle base diagnostics HUD (`FPS average`, `1% low`, frame latency)
- `F3 + G`: Toggle FPS graph pane (exclusive)
- `F3 + W`: Toggle secondary orbital debug camera pane (exclusive)
- `F3 + E`: Toggle entity diagnostics pane (`entity_count` + visible render classes)
- `F3 + S`: Toggle system stats pane (CPU, RAM, GPU name)
- `F3 + H`: Toggle controls/help pane
- `F3 + C`: Toggle culling counters pane
- `F3 + L`: Toggle frame timings pane

On macOS laptops, function keys may be captured by OS shortcuts (Mission Control, brightness, etc.).
If `F3` is intercepted by macOS, use the grave key (`) as a fallback debug modifier:

- `` ` ``: Toggle base diagnostics HUD
- `` ` + G/W/E/S/H/C/L ``: Same pane toggles as `F3 + ...`

Pane toggles are exclusive by design: enabling one pane replaces the previous pane.

## Recent Stability & Rendering Fixes (v0.1.2)
- **Frame Pacing**: Migrated engine update loop to `RedrawRequested` to eliminate camera stuttering and sync with display refresh rate.
- **Input Unification**: Unified `DeviceEvent` and `WindowEvent` mouse handling to fix double-accumulation choppiness.
- **Flat Shading Fallback**: Implemented mesh un-indexing in `canopy-pipeline` to ensure correct face normals for hard-edge geometry (like cubes) when source data lacks normals.
- **Sun Alignment**: Fixed inverted ray-tracing in `sky.wgsl` caused by Reversed-Z depth, aligning the visible sun disk with actual scene lighting.
- **Cursor Toggle**: Added `Escape` to release cursor and `Click` to re-grab for a better developer experience.

## License

Proprietary — All Rights Reserved. Wingit Team.
