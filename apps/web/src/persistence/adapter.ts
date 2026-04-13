import type { CityState } from '../types';
import { LocalSaveAdapter } from './local.adapter';

export interface SaveAdapter {
  save(cityName: string, state: CityState): Promise<void>;
  load(cityName: string): Promise<CityState | null>;
  listCities(): Promise<string[]>;
  delete(cityName: string): Promise<void>;
}

export const createSaveAdapter = (): SaveAdapter => new LocalSaveAdapter();
