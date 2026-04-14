import type { CityState, ServiceType, Tile } from '../types';

export const SERVICE_COVERAGE_RADIUS: Record<ServiceType, number> = {
  fire: 30,
  police: 25,
  health: 20,
  education: 15,
  power: Number.POSITIVE_INFINITY,
  water: 40
};

const distance = (from: Tile, to: Tile): number => Math.hypot(from.x - to.x, from.z - to.z);

const getCardinalNeighborIds = (tile: Tile): string[] => [
  `${tile.x + 1}_${tile.z}`,
  `${tile.x - 1}_${tile.z}`,
  `${tile.x}_${tile.z + 1}`,
  `${tile.x}_${tile.z - 1}`
];

const clamp01 = (value: number): number => Math.max(0, Math.min(1, value));

export class ServicesSystem {
  private city: CityState;

  private readonly eventTarget: EventTarget;

  constructor(initialCity: CityState, eventTarget: EventTarget = window) {
    this.city = initialCity;
    this.eventTarget = eventTarget;
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

    this.eventTarget.dispatchEvent(
      new CustomEvent('zonable:service:placed', {
        detail: { tileId, service }
      })
    );
  }

  getState(): CityState {
    return this.city;
  }

  private applyCoverage(city: CityState): CityState {
    const serviceTiles = Object.values(city.tiles).filter((tile) => tile.serviceIds.length > 0);
    const poweredRoadTiles = this.computePoweredRoadTiles(city, serviceTiles);
    const nextTiles: Record<string, Tile> = {};

    for (const tile of Object.values(city.tiles)) {
      const covered =
        poweredRoadTiles.has(tile.id) ||
        serviceTiles.some((serviceTile) =>
          serviceTile.serviceIds.some((service) => service !== 'power' && distance(serviceTile, tile) <= SERVICE_COVERAGE_RADIUS[service])
        );

      nextTiles[tile.id] = {
        ...tile,
        landValue: clamp01(tile.landValue + (covered ? 0.1 : 0))
      };
    }

    const nextBuildings = Object.fromEntries(
      Object.entries(city.buildings).map(([id, building]) => {
        const tile = nextTiles[building.tileId];
        const covered = tile.landValue > city.tiles[building.tileId].landValue;
        return [id, { ...building, happiness: clamp01(building.happiness + (covered ? 0.01 : -0.05)) }];
      })
    );

    return { ...city, tiles: nextTiles, buildings: nextBuildings };
  }

  private computePoweredRoadTiles(city: CityState, serviceTiles: Tile[]): Set<string> {
    const powerTiles = serviceTiles.filter((tile) => tile.serviceIds.includes('power'));
    if (powerTiles.length === 0) {
      return new Set<string>();
    }

    const visited = new Set<string>();
    const queue: string[] = [];

    for (const powerTile of powerTiles) {
      if (powerTile.road !== 'none') {
        queue.push(powerTile.id);
      }

      getCardinalNeighborIds(powerTile).forEach((neighborId) => {
        const neighbor = city.tiles[neighborId];
        if (neighbor && neighbor.road !== 'none') {
          queue.push(neighbor.id);
        }
      });
    }

    while (queue.length > 0) {
      const currentId = queue.shift() as string;
      if (visited.has(currentId)) {
        continue;
      }

      const current = city.tiles[currentId];
      if (!current || current.road === 'none') {
        continue;
      }

      visited.add(currentId);
      getCardinalNeighborIds(current).forEach((neighborId) => {
        const neighbor = city.tiles[neighborId];
        if (neighbor && neighbor.road !== 'none' && !visited.has(neighbor.id)) {
          queue.push(neighbor.id);
        }
      });
    }

    return visited;
  }
}
