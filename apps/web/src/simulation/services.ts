import type { CityState, ServiceType, Tile } from '../types';

const COVERAGE_RADIUS: Record<ServiceType, number> = {
  fire: 30,
  police: 25,
  health: 20,
  education: 15,
  power: Number.POSITIVE_INFINITY,
  water: 40
};

const distance = (from: Tile, to: Tile): number => Math.hypot(from.x - to.x, from.z - to.z);

export class ServicesSystem {
  private city: CityState;

  constructor(initialCity: CityState) {
    this.city = initialCity;
  }

  async init(): Promise<void> {
    return Promise.resolve();
  }

  update(_dt: number): void {
    this.city = this.applyCoverage(this.city);
  }

  placeService(tileId: string, service: ServiceType): void {
    const tile = this.city.tiles[tileId];
    if (!tile || tile.serviceIds.includes(service)) {
      return;
    }

    this.city = {
      ...this.city,
      tiles: {
        ...this.city.tiles,
        [tileId]: { ...tile, serviceIds: [...tile.serviceIds, service] }
      }
    };
  }

  getState(): CityState {
    return this.city;
  }

  private applyCoverage(city: CityState): CityState {
    const serviceTiles = Object.values(city.tiles).filter((tile) => tile.serviceIds.length > 0);
    const nextTiles: Record<string, Tile> = {};

    for (const tile of Object.values(city.tiles)) {
      const covered = serviceTiles.some((serviceTile) =>
        serviceTile.serviceIds.some((service) => distance(serviceTile, tile) <= COVERAGE_RADIUS[service])
      );

      nextTiles[tile.id] = {
        ...tile,
        landValue: Math.max(0, Math.min(1, tile.landValue + (covered ? 0.1 : 0)))
      };
    }

    const nextBuildings = Object.fromEntries(
      Object.entries(city.buildings).map(([id, building]) => {
        const tile = nextTiles[building.tileId];
        const covered = tile.landValue > city.tiles[building.tileId].landValue;
        return [id, { ...building, happiness: Math.max(0, Math.min(1, building.happiness + (covered ? 0.01 : -0.05))) }];
      })
    );

    return { ...city, tiles: nextTiles, buildings: nextBuildings };
  }
}
