import { BULLDOZE_COST_PER_TILE } from '../config/simulation.params';
import type { GridSystem } from '../simulation/grid';

export class BulldozeTool {
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

  clear(centerX: number, centerZ: number, brushSize: number): number {
    let cleared = 0;
    for (let dz = -brushSize + 1; dz < brushSize; dz += 1) {
      for (let dx = -brushSize + 1; dx < brushSize; dx += 1) {
        this.grid.demolish(centerX + dx, centerZ + dz);
        cleared += 1;
      }
    }
    return cleared * BULLDOZE_COST_PER_TILE;
  }
}
