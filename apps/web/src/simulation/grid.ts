import { DEFAULT_PARAMS } from '../config/simulation.params';
import type { BudgetState, Building, CityState, DemandState, RoadType, Tile, ZoneType } from '../types';

export const GRID_EVENTS = {
  zoneChanged: 'zonable:grid:zone-changed',
  roadChanged: 'zonable:grid:road-changed',
  buildingSpawned: 'zonable:grid:building-spawned',
  tileDemolished: 'zonable:grid:tile-demolished',
  elevationChanged: 'zonable:grid:elevation-changed'
} as const;

type GridEventName = (typeof GRID_EVENTS)[keyof typeof GRID_EVENTS];

export interface GridEventPayloadMap {
  [GRID_EVENTS.zoneChanged]: { tileId: string; zone: ZoneType };
  [GRID_EVENTS.roadChanged]: { tileId: string; road: RoadType };
  [GRID_EVENTS.buildingSpawned]: { tileId: string; building: Building };
  [GRID_EVENTS.tileDemolished]: { tileId: string };
  [GRID_EVENTS.elevationChanged]: { tileId: string; elevation: number };
}

const TILE_SIZE = 10;

const tileId = (x: number, z: number): string => `${x}_${z}`;

const createTile = (x: number, z: number): Tile => ({
  id: tileId(x, z),
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

const createDemandState = (): DemandState => ({ residential: 0.5, commercial: 0.5, industrial: 0.5 });

const createBudgetState = (): BudgetState => ({
  balance: 150_000,
  income: 0,
  expenses: 0,
  taxRates: { residential: 0.1, commercial: 0.1, industrial: 0.1 }
});

const clamp01 = (value: number): number => Math.max(0, Math.min(1, value));

export class GridSystem {
  private readonly eventTarget: EventTarget;

  private readonly width: number;

  private readonly depth: number;

  private state: CityState;

  constructor(cityName: string, width = 150, depth = 150, eventTarget: EventTarget = window) {
    this.width = width;
    this.depth = depth;
    this.eventTarget = eventTarget;
    this.state = createEmptyCity(cityName, width, depth);
  }

  async init(): Promise<void> {
    return Promise.resolve();
  }

  update(dt: number): void {
    this.state = { ...this.state, gameTime: this.state.gameTime + dt };
  }

  getState(): CityState {
    return this.state;
  }

  setState(state: CityState): void {
    this.state = state;
  }

  getTile(x: number, z: number): Tile | null {
    return this.state.tiles[tileId(x, z)] ?? null;
  }

  setZone(x: number, z: number, zone: ZoneType): void {
    const id = tileId(x, z);
    const tile = this.state.tiles[id];
    if (!tile || tile.zone === zone || tile.road !== 'none') {
      return;
    }

    const nextTile: Tile = { ...tile, zone, buildingId: zone === 'none' ? null : tile.buildingId };
    this.state = { ...this.state, tiles: { ...this.state.tiles, [id]: nextTile } };
    if (zone === 'none') {
      this.demolish(x, z);
      return;
    }
    this.emit(GRID_EVENTS.zoneChanged, { tileId: id, zone });
  }

  setRoad(x: number, z: number, type: RoadType): boolean {
    const id = tileId(x, z);
    const tile = this.state.tiles[id];
    if (!tile || tile.road === type) {
      return false;
    }

    const neighbors = this.getNeighborTiles(x, z);
    const blocked = neighbors.some((neighbor) => Math.abs(neighbor.elevation - tile.elevation) > 1);
    if (blocked || (tile.road !== 'none' && type !== 'none')) {
      return false;
    }

    const nextTile: Tile = {
      ...tile,
      road: type,
      zone: type === 'none' ? tile.zone : 'none',
      buildingId: type === 'none' ? tile.buildingId : null
    };

    this.state = {
      ...this.state,
      tiles: { ...this.state.tiles, [id]: nextTile }
    };

    if (type !== 'none') {
      const segmentId = `road_${id}`;
      this.state = {
        ...this.state,
        roads: {
          ...this.state.roads,
          [segmentId]: { id: segmentId, startTile: id, endTile: id, type, nodes: [id] }
        }
      };
    } else {
      const nextRoads = { ...this.state.roads };
      delete nextRoads[`road_${id}`];
      this.state = { ...this.state, roads: nextRoads };
    }

    this.emit(GRID_EVENTS.roadChanged, { tileId: id, road: type });
    return true;
  }

  spawnBuilding(x: number, z: number): Building | null {
    const id = tileId(x, z);
    const tile = this.state.tiles[id];
    if (!tile || tile.zone === 'none' || tile.road !== 'none' || tile.elevation > 2 || tile.buildingId) {
      return null;
    }

    const building: Building = {
      id: `b_${id}`,
      tileId: id,
      type: tile.zone,
      level: 1,
      meshId: '',
      population: tile.zone === 'residential' ? 10 : tile.zone === 'commercial' ? 6 : 12,
      happiness: 0.7
    };

    this.state = {
      ...this.state,
      tiles: { ...this.state.tiles, [id]: { ...tile, buildingId: building.id } },
      buildings: { ...this.state.buildings, [building.id]: building },
      population: this.state.population + (tile.zone === 'residential' ? building.population : 0)
    };

    this.emit(GRID_EVENTS.buildingSpawned, { tileId: id, building });
    return building;
  }

  demolish(x: number, z: number): void {
    const id = tileId(x, z);
    const tile = this.state.tiles[id];
    if (!tile) {
      return;
    }

    const buildings = { ...this.state.buildings };
    if (tile.buildingId) {
      delete buildings[tile.buildingId];
    }

    const roads = { ...this.state.roads };
    delete roads[`road_${id}`];

    this.state = {
      ...this.state,
      buildings,
      roads,
      tiles: {
        ...this.state.tiles,
        [id]: { ...tile, road: 'none', zone: 'none', buildingId: null, serviceIds: [] }
      }
    };

    this.emit(GRID_EVENTS.tileDemolished, { tileId: id });
  }

  setElevation(x: number, z: number, elevation: number): void {
    const id = tileId(x, z);
    const tile = this.state.tiles[id];
    if (!tile) {
      return;
    }
    this.state = {
      ...this.state,
      tiles: { ...this.state.tiles, [id]: { ...tile, elevation } }
    };
    this.emit(GRID_EVENTS.elevationChanged, { tileId: id, elevation });
  }

  getRoadGraphAdjacency(): Record<string, string[]> {
    const adjacency: Record<string, string[]> = {};
    for (const tile of Object.values(this.state.tiles)) {
      if (tile.road === 'none') {
        continue;
      }
      adjacency[tile.id] = this.getNeighborTiles(tile.x, tile.z)
        .filter((neighbor) => neighbor.road !== 'none')
        .map((neighbor) => neighbor.id);
    }
    return adjacency;
  }

  createRoadGraphSharedBuffer(): SharedArrayBuffer | null {
    if (typeof SharedArrayBuffer === 'undefined') {
      return null;
    }

    const graph = this.getRoadGraphAdjacency();
    const rows = Object.entries(graph).flatMap(([from, to]) => to.map((target) => [this.tileIndex(from), this.tileIndex(target)]));
    const buffer = new SharedArrayBuffer(rows.length * Int32Array.BYTES_PER_ELEMENT * 2);
    const view = new Int32Array(buffer);
    rows.forEach(([from, to], i) => {
      view[i * 2] = from;
      view[i * 2 + 1] = to;
    });
    return buffer;
  }

  private tileIndex(id: string): number {
    const [xPart, zPart] = id.split('_');
    return Number(zPart) * this.width + Number(xPart);
  }

  private getNeighborTiles(x: number, z: number): Tile[] {
    const deltas = [
      [1, 0],
      [-1, 0],
      [0, 1],
      [0, -1]
    ];
    return deltas
      .map(([dx, dz]) => this.getTile(x + dx, z + dz))
      .filter((tile): tile is Tile => tile !== null);
  }

  private emit<T extends GridEventName>(type: T, payload: GridEventPayloadMap[T]): void {
    this.eventTarget.dispatchEvent(new CustomEvent(type, { detail: payload }));
  }
}

export const createEmptyCity = (name: string, width = 150, depth = 150): CityState => {
  const tiles: Record<string, Tile> = {};
  for (let z = 0; z < depth; z += 1) {
    for (let x = 0; x < width; x += 1) {
      const nextTile = createTile(x, z);
      // The tile size is part of world-space conversion and kept here for discoverability.
      nextTile.landValue = clamp01(0.45 + TILE_SIZE * 0.0005);
      tiles[nextTile.id] = nextTile;
    }
  }
  return {
    name,
    population: 0,
    tiles,
    buildings: {},
    roads: {},
    demand: createDemandState(),
    budget: createBudgetState(),
    gameTime: 0,
    savedAt: Date.now()
  };
};
