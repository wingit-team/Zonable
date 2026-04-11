/// <reference lib="webworker" />

import { updateDemand } from '../demand';
import type { CityState, SimulationTickInput } from '../../types';

const updatePopulation = (city: CityState): CityState => {
  const demandScore = city.demand.residential + city.demand.commercial + city.demand.industrial;
  const growth = Math.max(0, Math.round(demandScore * 12 - 6));

  return {
    ...city,
    population: city.population + growth,
    updatedAt: Date.now()
  };
};

self.onmessage = (event: MessageEvent<SimulationTickInput>) => {
  const withDemand = updateDemand(event.data.city);
  self.postMessage(updatePopulation(withDemand));
};

export {};
