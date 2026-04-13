import type { SimulationParams, ServiceType } from '../types';

export const DEFAULT_PARAMS: SimulationParams = {
  demandDecayRate: 0.02,
  buildingSpawnThreshold: 0.6,
  buildingUpgradeThreshold: 0.85,
  pollutionSpreadRate: 0.01,
  landValueRadius: 15,
  citizenCommuteMaxTiles: 40,
  trafficCongestionThreshold: 0.75
};

export const BUDGET_WEEK_MS = 10_000;
export const AUTOSAVE_INTERVAL_MS = 10 * 60 * 1000;
export const ROAD_AUTOSAVE_SEGMENT_DELTA = 5;
export const BULLDOZE_COST_PER_TILE = 10;
export const LOAN_INCREMENT = 25_000;
export const LOAN_INTEREST_RATE = 0.1;

export const SERVICE_WEEKLY_COSTS: Record<ServiceType, number> = {
  fire: 700,
  police: 600,
  health: 900,
  education: 800,
  power: 1500,
  water: 700
};

