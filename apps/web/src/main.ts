import { render } from 'solid-js/web';

import { createRenderer } from './engine/renderer';
import { createScene } from './engine/scene';
import { setupPostFx } from './engine/postfx';
import { createTerrain } from './engine/terrain';
import { persistenceAdapter } from './persistence/adapter';
import { createEmptyCity } from './simulation/grid';
import { App } from './ui/App';
import type { CityState } from './types';

const createCanvas = (): HTMLCanvasElement => {
  const canvas = document.createElement('canvas');
  canvas.id = 'zonable-canvas';
  canvas.style.width = '100%';
  canvas.style.height = '100%';
  canvas.style.display = 'block';
  document.body.prepend(canvas);
  return canvas;
};

const spawnWorker = <TInput, TOutput>(url: URL) => {
  const worker = new Worker(url, { type: 'module' });

  return {
    tick(input: TInput): Promise<TOutput> {
      return new Promise<TOutput>((resolve) => {
        worker.onmessage = (event: MessageEvent<TOutput>): void => {
          worker.onmessage = null;
          resolve(event.data);
        };
        worker.postMessage(input);
      });
    }
  };
};

const bootstrap = async (): Promise<void> => {
  const canvas = createCanvas();
  const engine = await createRenderer(canvas);
  const scene = createScene(engine, canvas);
  createTerrain(scene);
  setupPostFx(scene);

  const economyWorker = spawnWorker<{ city: CityState; deltaMs: number }, CityState>(
    new URL('./simulation/workers/economy.worker.ts', import.meta.url)
  );
  const trafficWorker = spawnWorker<{ city: CityState; deltaMs: number }, CityState>(
    new URL('./simulation/workers/traffic.worker.ts', import.meta.url)
  );
  const citizensWorker = spawnWorker<{ city: CityState; deltaMs: number }, CityState>(
    new URL('./simulation/workers/citizens.worker.ts', import.meta.url)
  );

  let city = (await persistenceAdapter.load('autosave')) ?? createEmptyCity('New Zonable', 64, 64);

  engine.runRenderLoop(async () => {
    city = await economyWorker.tick({ city, deltaMs: 16 });
    city = await trafficWorker.tick({ city, deltaMs: 16 });
    city = await citizensWorker.tick({ city, deltaMs: 16 });
    scene.render();
  });

  window.addEventListener('beforeunload', () => {
    void persistenceAdapter.save('autosave', city);
  });

  window.addEventListener('resize', () => {
    engine.resize();
  });

  render(() => App({}), document.getElementById('root') as HTMLElement);
};

void bootstrap();
