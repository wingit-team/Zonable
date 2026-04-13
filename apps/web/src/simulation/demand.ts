import { DEFAULT_PARAMS } from '../config/simulation.params';
import type { CityState, DemandState, Tile } from '../types';

const clamp01 = (value: number): number => Math.min(1, Math.max(0, value));

const decayTowardNeutral = (value: number, decayRate: number): number => {
  const delta = 0.5 - value;
  return clamp01(value + delta * decayRate);
};

const hasJobsWithoutHousing = (city: CityState): boolean => {
  const housing = Object.values(city.buildings)
    .filter((building) => building.type === 'residential')
    .reduce((sum, building) => sum + building.population, 0);
  const jobs = Object.values(city.buildings)
    .filter((building) => building.type === 'commercial' || building.type === 'industrial')
    .reduce((sum, building) => sum + building.population, 0);
  return jobs > housing;
};

const highLandValueEmptyTiles = (tiles: Tile[]): number => tiles.filter((tile) => tile.landValue >= 0.7 && tile.zone !== 'none' && !tile.buildingId).length;

export class DemandSystem {
  private city: CityState;

  constructor(initialCity: CityState) {
    this.city = initialCity;
  }

  async init(): Promise<void> {
    return Promise.resolve();
  }

  update(_dt: number): void {
    this.city = this.compute(this.city);
  }

  compute(city: CityState): CityState {
    const tiles = Object.values(city.tiles);
    const residentialBase = decayTowardNeutral(city.demand.residential, DEFAULT_PARAMS.demandDecayRate);
    const commercialBase = decayTowardNeutral(city.demand.commercial, DEFAULT_PARAMS.demandDecayRate);
    const industrialBase = decayTowardNeutral(city.demand.industrial, DEFAULT_PARAMS.demandDecayRate);

    const residentialBoost = (hasJobsWithoutHousing(city) ? 0.08 : 0) + Math.min(0.08, highLandValueEmptyTiles(tiles) * 0.0006);

    const residentialBuildings = Object.values(city.buildings).filter((building) => building.type === 'residential').length;
    const commercialBuildings = Object.values(city.buildings).filter((building) => building.type === 'commercial').length;
    const commercialRatio = residentialBuildings === 0 ? 0 : commercialBuildings / residentialBuildings;
    const commercialBoost = city.population > 0 && commercialRatio < 0.3 ? 0.1 : 0;

    const industrialPenalty = tiles.some((tile) => tile.zone === 'industrial' && this.hasPollutedResidentialNeighbor(city, tile)) ? 0.1 : 0;

    const demand: DemandState = {
      residential: clamp01(residentialBase + residentialBoost),
      commercial: clamp01(commercialBase + commercialBoost),
      industrial: clamp01(industrialBase - industrialPenalty)
    };

    return { ...city, demand };
  }

  getState(): CityState {
    return this.city;
  }

  private hasPollutedResidentialNeighbor(city: CityState, tile: Tile): boolean {
    const neighbors = [
      city.tiles[`${tile.x + 1}_${tile.z}`],
      city.tiles[`${tile.x - 1}_${tile.z}`],
      city.tiles[`${tile.x}_${tile.z + 1}`],
      city.tiles[`${tile.x}_${tile.z - 1}`]
    ].filter((candidate): candidate is Tile => Boolean(candidate));

    return neighbors.some((neighbor) => neighbor.zone === 'residential' && neighbor.pollution > 0.5);
  }
}
