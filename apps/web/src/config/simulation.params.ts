import type { SimulationParams } from '../types';

export const DEFAULT_SIMULATION_PARAMS: SimulationParams = {
  demandDecayRate: 0.02,
  buildingSpawnThreshold: 0.6,
  buildingUpgradeThreshold: 0.8,
  pollutionSpreadRate: 0.04,
  landValueRadius: 3,
  citizenCommuteMaxTiles: 24,
  trafficCongestionThreshold: 0.75
};
