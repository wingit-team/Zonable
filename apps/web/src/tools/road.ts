import type { Vector3 } from '@babylonjs/core/Maths/math.vector';
import type { TerrainSystem } from '../engine/terrain';
import type { GridSystem } from '../simulation/grid';

export class RoadTool {
  private readonly grid: GridSystem;

  private readonly terrain: TerrainSystem;

  private startTileId: string | null = null;

  constructor(grid: GridSystem, terrain: TerrainSystem) {
    this.grid = grid;
    this.terrain = terrain;
  }

  async init(): Promise<void> {
    return Promise.resolve();
  }

  update(_dt: number): void {
    // Tool is input-driven in v1.
  }

  begin(tileId: string): void {
    this.startTileId = tileId;
  }

  preview(path: Vector3[]): void {
    this.terrain.createRoadRibbon(path);
  }

  commit(endTileId: string): number {
    if (!this.startTileId) {
      return 0;
    }

    const [sx, sz] = this.startTileId.split('_').map(Number);
    const [ex, ez] = endTileId.split('_').map(Number);
    const dx = Math.sign(ex - sx);
    const dz = Math.sign(ez - sz);
    let x = sx;
    let z = sz;
    let placed = 0;

    while (x !== ex || z !== ez) {
      if (this.grid.setRoad(x, z, 'two_lane')) {
        placed += 1;
      }
      x += dx;
      z += dz;
    }

    if (this.grid.setRoad(ex, ez, 'two_lane')) {
      placed += 1;
    }

    this.startTileId = null;
    return placed;
  }
}
