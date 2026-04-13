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

  sculpt(tileId: string, amount: number): void {
    const [x, z] = tileId.split('_').map(Number);
    const tile = this.grid.getTile(x, z);
    if (!tile) {
      return;
    }
    this.grid.setElevation(x, z, tile.elevation + amount);
  }
}
