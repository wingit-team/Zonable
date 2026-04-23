# CanScript API Reference

This document covers the Python API exposed by the `canopy` module.

## Core Module: `canopy`

### Math Types
High-performance math types backed by the Rust `glam` library.

#### `Vec3(x=0.0, y=0.0, z=0.0)`
- **Properties**: `x`, `y`, `z` (get/set)
- **Methods**:
    - `length() -> float`
    - `normalized() -> Vec3`
    - `distance(other: Vec3) -> float`
    - `dot(other: Vec3) -> float`
    - `cross(other: Vec3) -> Vec3`
    - `lerp(other: Vec3, t: float) -> Vec3`
- **Operators**: `+`, `-`, `*` (scalar), `/` (scalar), `==`, `iter()`

#### `Quat(x=0.0, y=0.0, z=0.0, w=1.0)`
- **Methods**:
    - `identity() -> Quat` (static)
    - `from_axis_angle(axis: Vec3, radians: float) -> Quat` (static)
    - `from_euler_xyz(p, y, r) -> Quat` (static)
    - `rotate_vec(v: Vec3) -> Vec3`
    - `slerp(other: Quat, t: float) -> Quat`

#### `Color(r=1.0, g=1.0, b=1.0, a=1.0)`
- **Methods**:
    - `from_hex(hex_str: str) -> Color` (static)
    - `white()`, `black()`, `red()` (static helpers)

---

### ECS Access: `canopy.world`
The `world` object is a global singleton available to all scripts.

- `world.spawn() -> Entity`: Spawns a new entity.
- `world.despawn(entity: Entity)`: Removes an entity.
- `world.add(entity, component_instance)`: Adds a component.
- `world.get(entity, ComponentClass) -> Instance`: Returns a component instance or None.
- `world.remove(entity, "ComponentName")`: Removes a component.
- `world.load_mesh(path: str) -> int`: (Phase 2) Asynchronously loads a mesh and returns a handle ID.

---

### Decorators
Register Python logic with the engine loop.

#### `@on_tick(rate_hz=0)`
Registers a function to be called every frame (if `rate_hz=0`) or at a specific frequency.
```python
@on_tick(rate_hz=60)
def my_logic(dt):
    print(f"Ticking with delta {dt}")
```

#### `@on_event(EventClass)`
Registers a handler for specific simulation events.
```python
from canopy.sim import EarthquakeEvent

@on_event(EarthquakeEvent)
def handle_quake(event):
    print(f"Epicenter: {event.epicenter}")
```

#### `@on_init`
Called once after all scripts are loaded but before the main loop starts.

---

## Submodule: `canopy.components`

### `Transform`
- `position: Vec3`
- `rotation: Quat`
- `scale: Vec3`

### `Mesh`
- `asset: str` (Path to `.canasset`)
- `lod_bias: float`
- `cast_shadow: bool`
- `receive_shadow: bool`

### `BuildingData`
- `zone_id: int`
- `capacity: int`
- `occupancy: int`
- `construction_progress: float` (0.0 to 1.0)
- `health: float`

### `Zone`
- `zone_type: str`
- `zone_id: int`
- `tier: int`
- `damage: float`
