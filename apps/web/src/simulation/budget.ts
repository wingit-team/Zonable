import type { BudgetState, CityState } from '../types';

const residentialIncome = (population: number, rate: number): number => population * 2 * rate;
const commercialIncome = (population: number, rate: number): number => population * 1.5 * rate;
const industrialIncome = (population: number, rate: number): number => population * 1.3 * rate;

export const updateBudget = (city: CityState): CityState => {
  const taxIncome =
    residentialIncome(city.population, city.budget.taxRates.residential) +
    commercialIncome(city.population, city.budget.taxRates.commercial) +
    industrialIncome(city.population, city.budget.taxRates.industrial);

  const upkeep = Object.keys(city.roads).length * 4 + Object.keys(city.buildings).length * 8;

  const budget: BudgetState = {
    ...city.budget,
    income: taxIncome,
    expenses: upkeep,
    balance: city.budget.balance + taxIncome - upkeep
  };

  return {
    ...city,
    budget,
    updatedAt: Date.now()
  };
};
