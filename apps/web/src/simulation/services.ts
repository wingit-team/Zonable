import type { CityState, ServiceType, Tile } from '../types';

export const assignServiceToTile = (city: CityState, tileId: string, service: ServiceType): CityState => {
  const tile = city.tiles[tileId];
  if (!tile || tile.serviceIds.includes(service)) {
    return city;
  }

  const nextTile: Tile = { ...tile, serviceIds: [...tile.serviceIds, service] };

  return {
    ...city,
    tiles: {
      ...city.tiles,
      [tileId]: nextTile
    },
    updatedAt: Date.now()
  };
};
