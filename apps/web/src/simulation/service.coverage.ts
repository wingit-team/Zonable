import { SERVICE_COVERAGE_RADIUS } from './services';
import type { CityState, ServiceType, Tile } from '../types';

export type ServiceCoverageState = Record<ServiceType, boolean>;

const getNeighborIds = (tile: Tile): string[] => [
  `${tile.x + 1}_${tile.z}`,
  `${tile.x - 1}_${tile.z}`,
  `${tile.x}_${tile.z + 1}`,
  `${tile.x}_${tile.z - 1}`
];

const isPowerCovered = (city: CityState, tile: Tile): boolean => {
  if (tile.serviceIds.includes('power')) {
    return true;
  }

  const powerTiles = Object.values(city.tiles).filter((candidate) => candidate.serviceIds.includes('power'));
  if (powerTiles.length === 0 || tile.road === 'none') {
    return false;
  }

  const visited = new Set<string>();
  const queue: string[] = [tile.id];

  while (queue.length > 0) {
    const currentId = queue.shift() as string;
    if (visited.has(currentId)) {
      continue;
    }

    const current = city.tiles[currentId];
    if (!current || current.road === 'none') {
      continue;
    }

    if (current.serviceIds.includes('power')) {
      return true;
    }

    visited.add(currentId);
    getNeighborIds(current).forEach((neighborId) => {
      const neighbor = city.tiles[neighborId];
      if (neighbor && neighbor.road !== 'none' && !visited.has(neighbor.id)) {
        queue.push(neighbor.id);
      }
    });
  }

  return false;
};

export const getServiceCoverageForTile = (city: CityState, tileId: string): ServiceCoverageState => {
  const tile = city.tiles[tileId];
  if (!tile) {
    return {
      fire: false,
      police: false,
      health: false,
      education: false,
      power: false,
      water: false
    };
  }

  const serviceTiles = Object.values(city.tiles).filter((candidate) => candidate.serviceIds.length > 0);
  const within = (service: Exclude<ServiceType, 'power'>): boolean =>
    serviceTiles.some((serviceTile) =>
      serviceTile.serviceIds.includes(service) && Math.hypot(serviceTile.x - tile.x, serviceTile.z - tile.z) <= SERVICE_COVERAGE_RADIUS[service]
    );

  return {
    fire: within('fire'),
    police: within('police'),
    health: within('health'),
    education: within('education'),
    water: within('water'),
    power: isPowerCovered(city, tile)
  };
};

