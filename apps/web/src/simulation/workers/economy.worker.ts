/// <reference lib="webworker" />

import type { BudgetState, CityState, WorkerMessage, ZoneType } from '../../types';

type EconomyTickPayload = {
  budget: BudgetState;
  city: CityState;
};

type EconomyResultPayload = {
  budget: BudgetState;
  happinessDelta: Record<Exclude<ZoneType, 'none'>, number>;
};

const computePopulationPerZone = (city: CityState): Record<Exclude<ZoneType, 'none'>, number> => {
  const totals = { residential: 0, commercial: 0, industrial: 0 };
  Object.values(city.buildings).forEach((building) => {
    if (building.type === 'none') {
      return;
    }
    totals[building.type] += building.population;
  });
  return totals;
};

self.onmessage = (event: MessageEvent<WorkerMessage<EconomyTickPayload>>): void => {
  if (event.data.type !== 'ECONOMY_TICK') {
    return;
  }

  const { city, budget } = event.data.payload;
  const totals = computePopulationPerZone(city);
  const income =
    totals.residential * budget.taxRates.residential +
    totals.commercial * budget.taxRates.commercial +
    totals.industrial * budget.taxRates.industrial;
  const serviceExpenses = Object.values(city.tiles).reduce((sum, tile) => sum + tile.serviceIds.length * 80, 0);
  const nextBudget: BudgetState = { ...budget, income, expenses: serviceExpenses, balance: budget.balance + income - serviceExpenses };

  if (nextBudget.balance < 0) {
    const warning: WorkerMessage<null> = { type: 'BANKRUPTCY_WARNING', payload: null };
    self.postMessage(warning);
    return;
  }

  const result: WorkerMessage<EconomyResultPayload> = {
    type: 'ECONOMY_RESULT',
    payload: {
      budget: nextBudget,
      happinessDelta: {
        residential: nextBudget.taxRates.residential > 0.15 ? -0.02 : 0.01,
        commercial: nextBudget.taxRates.commercial > 0.15 ? -0.02 : 0.01,
        industrial: nextBudget.taxRates.industrial > 0.15 ? -0.02 : 0.01
      }
    }
  };

  self.postMessage(result);
};

export {};
