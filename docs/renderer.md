# Canopy Renderer Architecture

The `canopy-renderer` is a high-performance graphics engine built on top of `wgpu`. It uses a **Hybrid Deferred + Forward** rendering path to balance high entity counts with visual fidelity.

## Core Components

### 1. RenderContext
The primary interface to the GPU. It manages the `wgpu::Device`, `wgpu::Queue`, and the `wgpu::Surface`. It is initialized during the `App` startup and stored as a global resource in the ECS.

### 2. GpuResourceManager
Manages all GPU-side allocations (Buffers, Textures, Bind Groups).
- **LRU Cache**: Automatically evicts unused assets when approaching the memory limit.
- **2GB Hard Limit**: Default memory budget for vertex/index/texture data to ensure stability on lower-end hardware.
- **Streaming**: Supports asynchronous data upload to prevent frame stutters during world traversal.

### 3. StandardPipeline
Encapsulates the multiple `wgpu::RenderPipeline` objects required for the hybrid path:
- **GBuffer Pipeline**: Renders Position, Normal, and Material properties to three 32-bit/16-bit float targets.
- **Lighting Pipeline**: A fullscreen pass that computes PBR lighting by reading the G-Buffer.
- **Forward Pipeline**: Handles transparent objects, particles, and UI elements.

## Rendering Pipeline Flow

### Phase 1: Extraction
The `render_system` queries the ECS for all entities with `Mesh` and `Transform` components. It evaluates:
- **Frustum Culling**: Entities outside the camera view are discarded.
- **LoD Selection**: Chooses the appropriate mesh level-of-detail (0-3) based on distance.
- **Batching**: Entities sharing the same mesh and material are grouped for instanced drawing.

### Phase 2: Execution
1.  **G-Buffer Pass**: Opaque geometry is drawn to high-precision textures.
2.  **Lighting Pass**: Global illumination and local lights are applied to the G-Buffer.
3.  **Forward Pass**: Transparents are drawn on top of the lit scene.
4.  **UI Pass**: Final overlay rendering.

## Shaders
Canopy uses **WGSL** (WebGPU Shading Language) for cross-platform compatibility.
- `gbuffer.wgsl`: Outputting `pos`, `norm`, `mat`.
- `lighting.wgsl`: PBR calculation with Cook-Torrance BRDF.
- `forward.wgsl`: Standard alpha-blending pass.
- `sky.wgsl`: Procedural sky with sun disk, matching the engine's `sun_direction`.

## Coordinate System
- **Right-Handed**: +X right, +Y up, -Z forward (matches `glam` defaults).
- **Reversed-Z**: Uses a floating-point depth buffer with reversed range (1.0 to 0.0) for significantly improved depth precision at long distances. Note: This requires inverting ray directions in full-screen passes (like `sky.wgsl`) since `near_world` (depth 1.0) is in front of `far_world` (depth 0.0).
