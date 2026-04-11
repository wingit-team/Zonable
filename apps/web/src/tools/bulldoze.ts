import type { CityState, Tile } from '../types';

export const bulldozeTile = (city: CityState, tileId: string): CityState => {
  const tile = city.tiles[tileId];
  if (!tile) {
    return city;
  }

  const nextTile: Tile = {
    ...tile,
    zone: 'none',
    road: 'none',
    buildingId: null,
    serviceIds: []
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
