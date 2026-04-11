import '@babylonjs/core/Lights/Shadows/shadowGeneratorSceneComponent';

import { ArcRotateCamera } from '@babylonjs/core/Cameras/arcRotateCamera';
import { HemisphericLight } from '@babylonjs/core/Lights/hemisphericLight';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import { Color4 } from '@babylonjs/core/Maths/math.color';
import { Scene } from '@babylonjs/core/scene';
import type { Engine } from '@babylonjs/core/Engines/engine';
import type { WebGPUEngine } from '@babylonjs/core/Engines/webgpuEngine';

export const createScene = (engine: Engine | WebGPUEngine, canvas: HTMLCanvasElement): Scene => {
  const scene = new Scene(engine);
  scene.clearColor = new Color4(0.05, 0.08, 0.12, 1);

  const camera = new ArcRotateCamera('city-camera', Math.PI / 4, 1, 160, new Vector3(32, 0, 32), scene);
  camera.attachControl(canvas, true);
  camera.lowerRadiusLimit = 20;
  camera.upperRadiusLimit = 240;

  const light = new HemisphericLight('sun-light', new Vector3(0.2, 1, 0.1), scene);
  light.intensity = 0.95;

  return scene;
};
