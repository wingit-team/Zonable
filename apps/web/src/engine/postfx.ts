import type { Scene } from '@babylonjs/core/scene';

export const setupPostFx = (scene: Scene): void => {
  scene.imageProcessingConfiguration.exposure = 1.1;
  scene.imageProcessingConfiguration.contrast = 1.2;
};
