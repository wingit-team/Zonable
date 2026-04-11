/// <reference lib="webworker" />

import { updateBudget } from '../budget';
import type { CityState, SimulationTickInput } from '../../types';

const runEconomyTick = (input: SimulationTickInput): CityState => {
  const updated = updateBudget(input.city);
  return {
    ...updated,
    tick: updated.tick + 1,
    updatedAt: Date.now()
  };
};

self.onmessage = (event: MessageEvent<SimulationTickInput>) => {
  const next = runEconomyTick(event.data);
  self.postMessage(next);
};

export {};
