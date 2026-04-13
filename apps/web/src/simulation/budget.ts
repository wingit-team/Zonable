import { BUDGET_WEEK_MS, LOAN_INCREMENT, LOAN_INTEREST_RATE, SERVICE_WEEKLY_COSTS } from '../config/simulation.params';
import type { BudgetState, CityState, ServiceType, ZoneType } from '../types';

type LoanState = {
  principal: number;
  weeksNegative: number;
};

const LEVEL_MULTIPLIER: Record<number, number> = { 1: 1, 2: 1.45, 3: 1.9 };

const clampTaxRate = (value: number): number => Math.max(0, Math.min(0.2, value));

export class BudgetSystem {
  private city: CityState;

  private loanState: LoanState;

  private accumulatedMs = 0;

  constructor(initialCity: CityState) {
    this.city = initialCity;
    this.loanState = { principal: 0, weeksNegative: 0 };
  }

  async init(): Promise<void> {
    return Promise.resolve();
  }

  update(dt: number): void {
    this.accumulatedMs += dt;
    while (this.accumulatedMs >= BUDGET_WEEK_MS) {
      this.accumulatedMs -= BUDGET_WEEK_MS;
      this.city = this.tickWeek(this.city);
    }
  }

  setTaxRate(zone: ZoneType, rate: number): void {
    if (zone === 'none') {
      return;
    }

    this.city = {
      ...this.city,
      budget: {
        ...this.city.budget,
        taxRates: {
          ...this.city.budget.taxRates,
          [zone]: clampTaxRate(rate)
        }
      }
    };
  }

  borrowLoan(): void {
    this.loanState.principal += LOAN_INCREMENT;
    this.city = {
      ...this.city,
      budget: { ...this.city.budget, balance: this.city.budget.balance + LOAN_INCREMENT }
    };
  }

  getState(): CityState {
    return this.city;
  }

  private tickWeek(city: CityState): CityState {
    const income = this.computeIncome(city);
    const expenses = this.computeServiceExpenses(city);
    const interest = this.loanState.principal > 0 ? this.loanState.principal * LOAN_INTEREST_RATE : 0;
    const repayment = this.loanState.principal > 0 ? Math.min(this.loanState.principal, Math.max(0, income * 0.25)) : 0;

    this.loanState.principal = Math.max(0, this.loanState.principal - repayment + interest);

    const nextBudget: BudgetState = {
      ...city.budget,
      income,
      expenses: expenses + interest,
      balance: city.budget.balance + income - expenses - interest - repayment
    };

    this.loanState.weeksNegative = nextBudget.balance < 0 ? this.loanState.weeksNegative + 1 : 0;
    if (this.loanState.weeksNegative >= 3) {
      window.dispatchEvent(new CustomEvent('zonable:budget:BANKRUPTCY_WARNING'));
    }

    return { ...city, budget: nextBudget };
  }

  private computeIncome(city: CityState): number {
    const totals: Record<Exclude<ZoneType, 'none'>, number> = {
      residential: 0,
      commercial: 0,
      industrial: 0
    };

    for (const building of Object.values(city.buildings)) {
      if (building.type === 'none') {
        continue;
      }
      const levelMultiplier = LEVEL_MULTIPLIER[building.level] ?? 1;
      totals[building.type] += building.population * levelMultiplier;
    }

    return (
      totals.residential * city.budget.taxRates.residential +
      totals.commercial * city.budget.taxRates.commercial +
      totals.industrial * city.budget.taxRates.industrial
    );
  }

  private computeServiceExpenses(city: CityState): number {
    const serviceCounts = new Map<ServiceType, number>();
    for (const tile of Object.values(city.tiles)) {
      tile.serviceIds.forEach((service) => {
        serviceCounts.set(service, (serviceCounts.get(service) ?? 0) + 1);
      });
    }

    let total = 0;
    for (const [service, count] of serviceCounts.entries()) {
      total += SERVICE_WEEKLY_COSTS[service] * count;
    }
    return total;
  }
}
