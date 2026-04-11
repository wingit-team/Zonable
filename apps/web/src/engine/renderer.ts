import { Engine } from '@babylonjs/core/Engines/engine';
import { WebGPUEngine } from '@babylonjs/core/Engines/webgpuEngine';

export const createRenderer = async (canvas: HTMLCanvasElement): Promise<Engine | WebGPUEngine> => {
  if (await WebGPUEngine.IsSupportedAsync) {
    const webgpu = new WebGPUEngine(canvas, {
      antialias: true,
      adaptToDeviceRatio: true
    });
    await webgpu.initAsync();
    return webgpu;
  }

  return new Engine(canvas, true, {
    preserveDrawingBuffer: true,
    stencil: true,
    disableWebGL2Support: false
  });
};
