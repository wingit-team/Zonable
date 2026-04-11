import type { CityState, Tile } from '../types';

export const raiseTerrain = (city: CityState, tileId: string, amount: number): CityState => {
  const tile = city.tiles[tileId];
  if (!tile) {
    return city;
  }

  const nextTile: Tile = {
    ...tile,
    elevation: tile.elevation + amount
  };

  return {
    ...city,
    tiles: {
      ...city.tiles,
      [tileId]: nextTile
    },
    updatedAt: Date.now()
  };
};
