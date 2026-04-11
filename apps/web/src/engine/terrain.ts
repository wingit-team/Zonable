import { MeshBuilder } from '@babylonjs/core/Meshes/meshBuilder';
import type { Scene } from '@babylonjs/core/scene';

export const createTerrain = (scene: Scene): void => {
  const ground = MeshBuilder.CreateGround('terrain-ground', { width: 128, height: 128, subdivisions: 32 }, scene);
  ground.position.y = 0;
};
