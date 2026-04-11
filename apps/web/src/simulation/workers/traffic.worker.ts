/// <reference lib="webworker" />

import type { CityState, SimulationTickInput, Tile } from '../../types';

const clamp01 = (value: number): number => Math.min(1, Math.max(0, value));

const updateTrafficPollution = (city: CityState): CityState => {
  const tiles = Object.fromEntries(
    Object.entries(city.tiles).map(([tileId, tile]): [string, Tile] => {
      const congestion = tile.road === 'none' ? 0 : tile.road === 'two_lane' ? 0.05 : tile.road === 'four_lane' ? 0.03 : 0.02;
      const pollution = clamp01(tile.pollution + congestion * city.params.pollutionSpreadRate);
      return [tileId, { ...tile, pollution }];
    })
  );

  return {
    ...city,
    tiles,
    updatedAt: Date.now()
  };
};

self.onmessage = (event: MessageEvent<SimulationTickInput>) => {
  self.postMessage(updateTrafficPollution(event.data.city));
};

export {};
