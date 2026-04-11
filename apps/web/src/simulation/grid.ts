import { DEFAULT_SIMULATION_PARAMS } from '../config/simulation.params';
import type { BudgetState, CityState, DemandState, Tile } from '../types';

const createTile = (x: number, z: number): Tile => ({
  id: `${x}_${z}`,
  x,
  z,
  elevation: 0,
  zone: 'none',
  road: 'none',
  buildingId: null,
  serviceIds: [],
  pollution: 0,
  landValue: 0.5
});

const createDemandState = (): DemandState => ({
  residential: 0.5,
  commercial: 0.5,
  industrial: 0.5
});

const createBudgetState = (): BudgetState => ({
  balance: 100_000,
  income: 0,
  expenses: 0,
  taxRates: {
    residential: 0.1,
    commercial: 0.12,
    industrial: 0.1
  }
});

export const createEmptyCity = (name: string, width: number, depth: number): CityState => {
  const tiles: Record<string, Tile> = {};

  for (let z = 0; z < depth; z += 1) {
    for (let x = 0; x < width; x += 1) {
      const tile = createTile(x, z);
      tiles[tile.id] = tile;
    }
  }

  return {
    name,
    population: 0,
    tiles,
    buildings: {},
    roads: {},
    citizens: {},
    demand: createDemandState(),
    budget: createBudgetState(),
    params: DEFAULT_SIMULATION_PARAMS,
    tick: 0,
    updatedAt: Date.now()
  };
};
