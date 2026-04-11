import type { CityState } from '../types';

const clamp01 = (value: number): number => Math.min(1, Math.max(0, value));

export const updateDemand = (city: CityState): CityState => {
  const decay = city.params.demandDecayRate;
  const residential = clamp01(city.demand.residential - decay * 0.8 + city.population / 500_000);
  const commercial = clamp01(city.demand.commercial - decay + city.population / 1_000_000);
  const industrial = clamp01(city.demand.industrial - decay * 0.6 + city.population / 800_000);

  return {
    ...city,
    demand: {
      residential,
      commercial,
      industrial
    }
  };
};
