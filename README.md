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
└── docs/                  # [Architecture](docs/architecture.md), [Python API](docs/canscript-api.md), [Asset Format](docs/canasset-format.md)
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

## Key Design Decisions

- **ECS**: Custom dense SoA storage with archetype grouping. No dynamic dispatch in hot path.
- **Simulation budget**: Entities outside view run at 4Hz heartbeat; statistical pools used beyond active radius.
- **Traffic**: Fluid dynamics flow field on road network — not per-agent pathfinding.
- **Assets**: `.canasset` is a memory-mappable binary format — zero runtime parsing overhead.
- **Chunk streaming**: Predictive pre-loading based on entity velocity + look direction.
- **GIL strategy**: Python systems dispatch through a `ScriptRunner` that batches inputs before acquiring the GIL.

## License

Proprietary — All Rights Reserved. Wingit Team.
