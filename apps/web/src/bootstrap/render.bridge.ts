import { Vector3 } from '@babylonjs/core/Maths/math.vector';

import { RendererSystem } from '../engine/renderer';
import { TerrainSystem } from '../engine/terrain';
import { GRID_EVENTS, GridSystem } from '../simulation/grid';
import type { Building, CityState, ZoneType } from '../types';

type SetCity = (value: CityState) => void;

const toWorld = (state: CityState, tileId: string): Vector3 => {
  const tile = state.tiles[tileId];
  return new Vector3(tile.x * 10 - 745, tile.elevation + 4, tile.z * 10 - 745);
};

export const setupRenderBridge = (grid: GridSystem, terrain: TerrainSystem, renderer: RendererSystem, setCity: SetCity): void => {
  const tileToBuildingId = new Map<string, string>();

  window.addEventListener(GRID_EVENTS.zoneChanged, (event) => {
    const detail = (event as CustomEvent<{ tileId: string; zone: ZoneType }>).detail;
    terrain.onZoneChanged(detail);
    if (detail.zone !== 'none') {
      const [x, z] = detail.tileId.split('_').map(Number);
      grid.spawnBuilding(x, z);
      setCity(grid.getState());
    }
  });

  window.addEventListener(GRID_EVENTS.buildingSpawned, (event) => {
    const detail = (event as CustomEvent<{ tileId: string; building: Building }>).detail;
    tileToBuildingId.set(detail.tileId, detail.building.id);
    renderer.spawnBuilding(detail.building, toWorld(grid.getState(), detail.tileId));
  });

  window.addEventListener(GRID_EVENTS.tileDemolished, (event) => {
    const tileId = (event as CustomEvent<{ tileId: string }>).detail.tileId;
    const buildingId = tileToBuildingId.get(tileId);
    if (!buildingId) {
      return;
    }
    renderer.demolishBuilding(buildingId);
    tileToBuildingId.delete(tileId);
  });

  window.addEventListener(GRID_EVENTS.elevationChanged, (event) => {
    terrain.onElevationChanged((event as CustomEvent<{ tileId: string; elevation: number }>).detail);
  });
};

