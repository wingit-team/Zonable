import type { CityState } from '../types';
import type { SaveAdapter } from './adapter';

const SAVE_PREFIX = 'zonable_city_';
const INDEX_KEY = 'zonable_index';
const MAX_SAVE_BYTES = 4 * 1024 * 1024;

const keyFor = (cityName: string): string => `${SAVE_PREFIX}${cityName}`;

const parseIndex = (raw: string | null): string[] => {
  if (!raw) {
    return [];
  }

  try {
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((name): name is string => typeof name === 'string') : [];
  } catch {
    return [];
  }
};

const getIndex = (): string[] => parseIndex(localStorage.getItem(INDEX_KEY));

const setIndex = (cityNames: string[]): void => {
  localStorage.setItem(INDEX_KEY, JSON.stringify(cityNames));
};

export class LocalSaveAdapter implements SaveAdapter {
  async save(cityName: string, state: CityState): Promise<void> {
    const payload = JSON.stringify({ ...state, savedAt: Date.now() });
    if (payload.length > MAX_SAVE_BYTES) {
      console.warn(`[Zonable] Save '${cityName}' exceeds 4MB (${payload.length} bytes).`);
    }

    localStorage.setItem(keyFor(cityName), payload);

    const index = getIndex();
    if (!index.includes(cityName)) {
      setIndex([...index, cityName]);
    }
  }

  async load(cityName: string): Promise<CityState | null> {
    const payload = localStorage.getItem(keyFor(cityName));
    if (!payload) {
      return null;
    }

    try {
      return JSON.parse(payload) as CityState;
    } catch {
      return null;
    }
  }

  async listCities(): Promise<string[]> {
    return getIndex();
  }

  async delete(cityName: string): Promise<void> {
    localStorage.removeItem(keyFor(cityName));
    setIndex(getIndex().filter((name) => name !== cityName));
  }
}
