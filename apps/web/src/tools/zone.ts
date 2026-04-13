import type { GridSystem } from '../simulation/grid';
import type { ZoneType } from '../types';

export class ZoneTool {
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

  paint(centerX: number, centerZ: number, zone: ZoneType, brushSize: number): void {
	for (let dz = -brushSize + 1; dz < brushSize; dz += 1) {
	  for (let dx = -brushSize + 1; dx < brushSize; dx += 1) {
		this.grid.setZone(centerX + dx, centerZ + dz, zone);
	  }
	}
  }
}
