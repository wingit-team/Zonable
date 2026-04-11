import type { CityState, SaveSlot } from '../types';

const SAVE_PREFIX = 'zonable-save:';

export interface CityStorageAdapter {
  save(slotId: string, city: CityState): Promise<void>;
  load(slotId: string): Promise<CityState | null>;
  list(): Promise<SaveSlot[]>;
  remove(slotId: string): Promise<void>;
}

const keyFor = (slotId: string): string => `${SAVE_PREFIX}${slotId}`;

export class LocalStorageAdapter implements CityStorageAdapter {
  async save(slotId: string, city: CityState): Promise<void> {
    const slot: SaveSlot = {
      id: slotId,
      city,
      savedAt: Date.now()
    };

    localStorage.setItem(keyFor(slotId), JSON.stringify(slot));
  }

  async load(slotId: string): Promise<CityState | null> {
    const raw = localStorage.getItem(keyFor(slotId));
    if (!raw) {
      return null;
    }

    const parsed = JSON.parse(raw) as SaveSlot;
    return parsed.city;
  }

  async list(): Promise<SaveSlot[]> {
    const saves: SaveSlot[] = [];

    for (let i = 0; i < localStorage.length; i += 1) {
      const key = localStorage.key(i);
      if (!key || !key.startsWith(SAVE_PREFIX)) {
        continue;
      }

      const raw = localStorage.getItem(key);
      if (!raw) {
        continue;
      }

      const save = JSON.parse(raw) as SaveSlot;
      saves.push(save);
    }

    return saves.sort((a, b) => b.savedAt - a.savedAt);
  }

  async remove(slotId: string): Promise<void> {
    localStorage.removeItem(keyFor(slotId));
  }
}
