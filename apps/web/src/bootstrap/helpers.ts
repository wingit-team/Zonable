import type { Scene } from '@babylonjs/core/scene';

import type { WorkerMessage } from '../types';

export const WORLD_TILE_SIZE = 10;
export const WORLD_GRID_SIZE = 150;

export const createCanvas = (): HTMLCanvasElement => {
  const canvas = document.createElement('canvas');
  canvas.id = 'zonable-canvas';
  canvas.style.position = 'fixed';
  canvas.style.inset = '0';
  canvas.style.width = '100%';
  canvas.style.height = '100%';
  canvas.style.display = 'block';
  canvas.style.zIndex = '0';
  document.body.prepend(canvas);
  return canvas;
};

export const pickTile = (scene: Scene): { x: number; z: number } | null => {
  const pick = scene.pick(scene.pointerX, scene.pointerY, (mesh) => mesh.id === 'terrain-ground');
  if (!pick?.hit || !pick.pickedPoint) {
    return null;
  }

  const x = Math.floor((pick.pickedPoint.x + (WORLD_GRID_SIZE * WORLD_TILE_SIZE) / 2) / WORLD_TILE_SIZE);
  const z = Math.floor((pick.pickedPoint.z + (WORLD_GRID_SIZE * WORLD_TILE_SIZE) / 2) / WORLD_TILE_SIZE);
  if (x < 0 || z < 0 || x >= WORLD_GRID_SIZE || z >= WORLD_GRID_SIZE) {
    return null;
  }

  return { x, z };
};

export const spawnWorker = <TInput, TOutput>(url: URL) => {
  const worker = new Worker(url, { type: 'module' });

  return {
    tick(message: WorkerMessage<TInput>): Promise<WorkerMessage<TOutput>> {
      return new Promise<WorkerMessage<TOutput>>((resolve) => {
        worker.onmessage = (event: MessageEvent<WorkerMessage<TOutput>>): void => {
          worker.onmessage = null;
          resolve(event.data);
        };
        worker.postMessage(message);
      });
    }
  };
};

