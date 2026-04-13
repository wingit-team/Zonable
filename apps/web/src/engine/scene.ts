import '@babylonjs/core/Lights/Shadows/shadowGeneratorSceneComponent';

import { ArcRotateCamera } from '@babylonjs/core/Cameras/arcRotateCamera';
import { Engine } from '@babylonjs/core/Engines/engine';
import { WebGPUEngine } from '@babylonjs/core/Engines/webgpuEngine';
import { DirectionalLight } from '@babylonjs/core/Lights/directionalLight';
import { HemisphericLight } from '@babylonjs/core/Lights/hemisphericLight';
import { Color3, Color4 } from '@babylonjs/core/Maths/math.color';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import { Scene } from '@babylonjs/core/scene';
import { ShadowGenerator } from '@babylonjs/core/Lights/Shadows/shadowGenerator';

type StartEngineResult = { engine: Engine | WebGPUEngine; scene: Scene };

const lerpColor = (a: Color3, b: Color3, t: number): Color3 => Color3.Lerp(a, b, t);

const easeInOut = (t: number): number => (1 - Math.cos(Math.PI * t)) * 0.5;

const phaseColor = (phase: number): Color3 => {
  const dawn = new Color3(1.0, 0.55, 0.35);
  const noon = new Color3(1.0, 1.0, 1.0);
  const dusk = new Color3(0.95, 0.45, 0.2);
  const night = new Color3(0.1, 0.16, 0.35);

  if (phase < 0.25) {
    return lerpColor(night, dawn, easeInOut(phase / 0.25));
  }
  if (phase < 0.5) {
    return lerpColor(dawn, noon, easeInOut((phase - 0.25) / 0.25));
  }
  if (phase < 0.75) {
    return lerpColor(noon, dusk, easeInOut((phase - 0.5) / 0.25));
  }
  return lerpColor(dusk, night, easeInOut((phase - 0.75) / 0.25));
};

export class SceneSystem {
  private readonly scene: Scene;

  private readonly camera: ArcRotateCamera;

  private readonly hemiLight: HemisphericLight;

  private readonly sunLight: DirectionalLight;

  private readonly shadowGenerator: ShadowGenerator;

  private elapsedMs = 0;

  constructor(scene: Scene, canvas: HTMLCanvasElement) {
    this.scene = scene;
    this.scene.clearColor = new Color4(0.19, 0.25, 0.38, 1);

    this.camera = new ArcRotateCamera('city-camera', -Math.PI / 4, Math.PI / 3.5, 80, Vector3.Zero(), scene);
    this.camera.lowerBetaLimit = 0.3;
    this.camera.upperBetaLimit = Math.PI / 2.2;
    this.camera.lowerRadiusLimit = 15;
    this.camera.upperRadiusLimit = 220;
    this.camera.panningSensibility = 70;
    this.camera.attachControl(canvas, true);

    this.hemiLight = new HemisphericLight('hemi-light', new Vector3(0, 1, 0), scene);
    this.hemiLight.intensity = 0.4;
    this.hemiLight.groundColor = new Color3(0.07, 0.09, 0.12);

    this.sunLight = new DirectionalLight('sun-light', new Vector3(-0.6, -1, -0.4), scene);
    this.sunLight.intensity = 1.2;
    this.sunLight.position = new Vector3(50, 80, 50);

    this.shadowGenerator = new ShadowGenerator(2048, this.sunLight);
    this.shadowGenerator.usePercentageCloserFiltering = true;
    this.shadowGenerator.setDarkness(0.35);
  }

  async init(): Promise<void> {
    return Promise.resolve();
  }

  update(dt: number): void {
    this.elapsedMs += dt;
    const dayPhase = (this.elapsedMs % (24 * 60 * 1000)) / (24 * 60 * 1000);
    const sunAngle = dayPhase * Math.PI * 2;
    const direction = new Vector3(Math.cos(sunAngle), -Math.max(0.15, Math.sin(sunAngle)), Math.sin(sunAngle));
    this.sunLight.direction = direction.normalize();
    this.sunLight.position = this.camera.target.add(this.sunLight.direction.scale(-120));

    const ambient = phaseColor(dayPhase);
    this.scene.clearColor = new Color4(ambient.r * 0.4, ambient.g * 0.45, ambient.b * 0.55, 1);
    this.hemiLight.groundColor = ambient.scale(0.15);
    this.hemiLight.diffuse = ambient;
  }

  getShadowGenerator(): ShadowGenerator {
    return this.shadowGenerator;
  }
}

export const startEngine = async (canvas: HTMLCanvasElement): Promise<StartEngineResult> => {
  let engine: Engine | WebGPUEngine;
  if (await WebGPUEngine.IsSupportedAsync) {
    const webGpuEngine = new WebGPUEngine(canvas, { antialias: true, adaptToDeviceRatio: true });
    await webGpuEngine.initAsync();
    engine = webGpuEngine;
  } else {
    engine = new Engine(canvas, true, { preserveDrawingBuffer: false, stencil: true, disableWebGL2Support: false });
  }
  return { engine, scene: new Scene(engine) };
};
