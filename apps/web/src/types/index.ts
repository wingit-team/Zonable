export type ZoneType = 'none' | 'residential' | 'commercial' | 'industrial';
export type RoadType = 'none' | 'two_lane' | 'four_lane' | 'highway';
export type ServiceType = 'fire' | 'police' | 'health' | 'education' | 'power' | 'water';

export interface Tile {
  id: string;
  x: number;
  z: number;
  elevation: number;
  zone: ZoneType;
  road: RoadType;
  buildingId: string | null;
  serviceIds: ServiceType[];
  pollution: number;
  landValue: number;
}

export interface Building {
  id: string;
  tileId: string;
  type: ZoneType;
  level: number;
  meshId: string;
  population: number;
  happiness: number;
}

export interface RoadSegment {
  id: string;
  startTile: string;
  endTile: string;
  type: RoadType;
  nodes: string[];
}

export interface CitizenAgent {
  id: string;
  homeTileId: string;
  workTileId: string | null;
  state: 'home' | 'commuting' | 'working' | 'returning';
  pathProgress: number;
  happiness: number;
}

export interface DemandState {
  residential: number;
  commercial: number;
  industrial: number;
}

export interface BudgetState {
  balance: number;
  income: number;
  expenses: number;
  taxRates: {
    residential: number;
    commercial: number;
    industrial: number;
  };
}

export interface SimulationParams {
  demandDecayRate: number;
  buildingSpawnThreshold: number;
  buildingUpgradeThreshold: number;
  pollutionSpreadRate: number;
  landValueRadius: number;
  citizenCommuteMaxTiles: number;
  trafficCongestionThreshold: number;
}

export interface CityState {
  name: string;
  population: number;
  tiles: Record<string, Tile>;
  buildings: Record<string, Building>;
  roads: Record<string, RoadSegment>;
  citizens: Record<string, CitizenAgent>;
  demand: DemandState;
  budget: BudgetState;
  params: SimulationParams;
  tick: number;
  updatedAt: number;
}

export interface SaveSlot {
  id: string;
  city: CityState;
  savedAt: number;
}

export interface SimulationTickInput {
  city: CityState;
  deltaMs: number;
}
