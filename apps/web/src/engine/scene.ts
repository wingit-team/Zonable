import {
  ArcRotateCamera,
  DefaultRenderingPipeline,
  DirectionalLight,
  Engine,
  HemisphericLight,
  ImageProcessingConfiguration,
  Scene,
  SSAO2RenderingPipeline,
  Vector3,
  WebGPUEngine,
} from "@babylonjs/core";

export type EngineStartResult = {
  engine: Engine;
  scene: Scene;
};

const createEngine = async (canvas: HTMLCanvasElement): Promise<Engine> => {
  if (await WebGPUEngine.IsSupportedAsync) {
    const webgpuEngine = new WebGPUEngine(canvas, {
      antialiasing: true,
    });

    await webgpuEngine.initAsync();
    return webgpuEngine;
  }

  return new Engine(canvas, true, {
    antialias: true,
    stencil: true,
  });
};

export const startEngine = async (
  canvas: HTMLCanvasElement,
): Promise<EngineStartResult> => {
  const engine = await createEngine(canvas);
  const scene = new Scene(engine);

  const camera = new ArcRotateCamera(
    "main-camera",
    -Math.PI / 2,
    Math.PI / 3,
    900,
    Vector3.Zero(),
    scene,
  );
  camera.attachControl(canvas, true);
  camera.lowerAlphaLimit = camera.alpha;
  camera.upperAlphaLimit = camera.alpha;
  camera.lowerBetaLimit = camera.beta;
  camera.upperBetaLimit = camera.beta;

  new HemisphericLight("hemisphere-light", new Vector3(0, 1, 0), scene);

  const sunLight = new DirectionalLight(
    "sun-light",
    new Vector3(-1, -2, -1),
    scene,
  );
  sunLight.position = new Vector3(250, 500, 250);

  const ssaoPipeline = new SSAO2RenderingPipeline(
    "ssao-pipeline",
    scene,
    {
      ssaoRatio: 0.5,
      blurRatio: 1,
    },
    [camera],
  );
  scene.postProcessRenderPipelineManager.attachCamerasToRenderPipeline(
    ssaoPipeline.name,
    [camera],
  );

  const defaultPipeline = new DefaultRenderingPipeline(
    "default-pipeline",
    true,
    scene,
    [camera],
  );
  defaultPipeline.bloomEnabled = true;
  defaultPipeline.imageProcessingEnabled = true;
  defaultPipeline.imageProcessing.toneMappingEnabled = true;
  defaultPipeline.imageProcessing.toneMappingType =
    ImageProcessingConfiguration.TONEMAPPING_ACES;

  engine.runRenderLoop(() => {
    scene.render();
  });

  window.addEventListener("resize", () => {
    engine.resize();
  });

  return { engine, scene };
};
