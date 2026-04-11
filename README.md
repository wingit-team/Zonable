# Zonable

Web-based 3D city builder built as a strict TypeScript monorepo with local-first persistence.

## Stack

- Babylon.js renderer (WebGPU with WebGL2 fallback)
- Solid.js HUD/UI
- Vite app bundling
- pnpm workspaces monorepo
- Simulation tick workers on Web Workers
- Rust/WASM pathfinding package (`packages/pathfinding`)

## Workspace layout

- `apps/web` - browser game client
- `packages/pathfinding` - Rust crate for pathfinding exports

## Local development

```bash
pnpm install
pnpm dev
```

## Build + checks

```bash
pnpm typecheck
pnpm build
```
