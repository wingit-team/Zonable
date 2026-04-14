import type { GridSystem } from '../simulation/grid';

export class TerrainTool {
  private readonly grid: GridSystem;

  constructor(grid: GridSystem) {
    this.grid = grid;
  }

  async init(): Promise<void> {
    return Promise.resolve();
  }

  update(_dt: number): void {
    // Tool is user input driven.
  }

  sculpt(tileId: string, amount: number, brushSize = 1): void {
    const [centerX, centerZ] = tileId.split('_').map(Number);
    for (let dz = -brushSize + 1; dz < brushSize; dz += 1) {
      for (let dx = -brushSize + 1; dx < brushSize; dx += 1) {
        const x = centerX + dx;
        const z = centerZ + dz;
        const tile = this.grid.getTile(x, z);
        if (!tile) {
          continue;
        }
        this.grid.setElevation(x, z, tile.elevation + amount);
      }
    }
  }
}
