import type { CityState, RoadType, Tile } from '../types';

export const paintRoad = (city: CityState, tileId: string, road: RoadType): CityState => {
  const tile = city.tiles[tileId];
  if (!tile) {
    return city;
  }

  const nextTile: Tile = { ...tile, road };

  return {
    ...city,
    tiles: {
      ...city.tiles,
      [tileId]: nextTile
    },
    updatedAt: Date.now()
  };
};
