import type { CityState, Tile, ZoneType } from '../types';

export const setZone = (city: CityState, tileId: string, zone: ZoneType): CityState => {
  const tile = city.tiles[tileId];
  if (!tile) {
    return city;
  }

  const nextTile: Tile = { ...tile, zone };

  return {
    ...city,
    tiles: {
      ...city.tiles,
      [tileId]: nextTile
    }
  };
};
