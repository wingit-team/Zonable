import { createSignal } from 'solid-js';
import { render } from 'solid-js/web';

import { PostFxSystem } from '../engine/postfx';
import { RendererSystem } from '../engine/renderer';
import { SceneSystem, startEngine } from '../engine/scene';
import { TerrainSystem } from '../engine/terrain';
import { AUTOSAVE_INTERVAL_MS, ROAD_AUTOSAVE_SEGMENT_DELTA } from '../config/simulation.params';
import { createSaveAdapter } from '../persistence/adapter';
import { BudgetSystem } from '../simulation/budget';
import { DemandSystem } from '../simulation/demand';
import { GridSystem, GRID_EVENTS } from '../simulation/grid';
import { ServicesSystem } from '../simulation/services';
import { BulldozeTool } from '../tools/bulldoze';
import { RoadTool } from '../tools/road';
import { TerrainTool } from '../tools/terrain.tool';
import { ZoneTool } from '../tools/zone';
import { App } from '../ui/App';
import type { CityState } from '../types';
import { createCanvas, pickTile, spawnWorker } from './helpers';
export const bootstrapApp = async (): Promise<void> => {
  const saveAdapter = createSaveAdapter();
  const canvas = createCanvas();
  const { engine, scene } = await startEngine(canvas);
  const grid = new GridSystem('New Zonable');
  await grid.init();
  const loadedCity = await saveAdapter.load('autosave');
  if (loadedCity) {
    grid.setState(loadedCity);
  }

  const sceneSystem = new SceneSystem(scene, canvas);
  const terrainSystem = new TerrainSystem(scene);
  const rendererSystem = new RendererSystem(scene);
  const postFxSystem = new PostFxSystem(scene);
  const demandSystem = new DemandSystem(grid.getState());
  const budgetSystem = new BudgetSystem(grid.getState());
  const servicesSystem = new ServicesSystem(grid.getState());
  const roadTool = new RoadTool(grid, terrainSystem);
  const zoneTool = new ZoneTool(grid);
  const bulldozeTool = new BulldozeTool(grid);
  const terrainTool = new TerrainTool(grid);
  await Promise.all([
    sceneSystem.init(),
    terrainSystem.init(),
    rendererSystem.init(),
    postFxSystem.init(),
    demandSystem.init(),
    budgetSystem.init(),
    servicesSystem.init(),
    roadTool.init(),
    zoneTool.init(),
    bulldozeTool.init(),
    terrainTool.init()
  ]);

  window.addEventListener(GRID_EVENTS.zoneChanged, (event) => {
    terrainSystem.onZoneChanged((event as CustomEvent<{ tileId: string; zone: 'none' | 'residential' | 'commercial' | 'industrial' }>).detail);
  });
  window.addEventListener(GRID_EVENTS.elevationChanged, (event) => {
    terrainSystem.onElevationChanged((event as CustomEvent<{ tileId: string; elevation: number }>).detail);
  });

  let roadChangesSinceLastSave = 0;
  window.addEventListener(GRID_EVENTS.roadChanged, () => {
    roadChangesSinceLastSave += 1;
  });

  const economyWorker = spawnWorker<{ budget: CityState['budget']; city: CityState }, { budget: CityState['budget']; happinessDelta: Record<'residential' | 'commercial' | 'industrial', number> } | null>(
    new URL('../simulation/workers/economy.worker.ts', import.meta.url)
  );
  const trafficWorker = spawnWorker<{ adjacency: Record<string, string[]>; citizenCounts: Record<string, number> }, Record<string, number>>(
    new URL('../simulation/workers/traffic.worker.ts', import.meta.url)
  );
  const citizensWorker = spawnWorker<{ city: CityState; gameTime: number }, { positions: Array<{ id: string; progress: number; state: string }>; happinessByTile: Record<string, number> }>(
    new URL('../simulation/workers/citizens.worker.ts', import.meta.url)
  );
  const [city, setCity] = createSignal(grid.getState());
  const [activeTool, setActiveTool] = createSignal<'road' | 'zone' | 'bulldoze' | 'terrain' | 'services'>('zone');
  const [selectedZone, setSelectedZone] = createSignal<'residential' | 'commercial' | 'industrial'>('residential');
  const [selectedService, setSelectedService] = createSignal<'fire' | 'police' | 'health' | 'education' | 'power' | 'water'>('fire');
  const [brushSize, setBrushSize] = createSignal(1);
  const [saveState, setSaveState] = createSignal<'idle' | 'saving' | 'saved'>('idle');
  const [notifications, setNotifications] = createSignal<string[]>(['Welcome to Zonable']);
  const [graphics, setGraphics] = createSignal({ ssao: true, bloom: true, shadows: true, dof: false });
  const [simulationSpeed, setSimulationSpeed] = createSignal<0 | 1 | 2 | 3>(1);
  const [audioVolume, setAudioVolume] = createSignal(0.5);
  const [selectedTileId, setSelectedTileId] = createSignal<string | null>(null);

  let pendingRoadStart: string | null = null;
  const persist = async (name: string): Promise<void> => {
    setSaveState('saving');
    await saveAdapter.save(name, grid.getState());
    setSaveState('saved');
    window.setTimeout(() => setSaveState('idle'), 1200);
    roadChangesSinceLastSave = 0;
  };

  window.setInterval(() => void persist('autosave'), AUTOSAVE_INTERVAL_MS);
  window.addEventListener('zonable:service:placed', () => void persist('autosave'));

  scene.onPointerDown = () => {
    const picked = pickTile(scene);
    if (!picked) {
      return;
    }

    const tileId = `${picked.x}_${picked.z}`;
    setSelectedTileId(tileId);
    const tool = activeTool();
    if (tool === 'zone') {
      zoneTool.paint(picked.x, picked.z, selectedZone(), brushSize());
    }
    if (tool === 'bulldoze') {
      const cost = bulldozeTool.clear(picked.x, picked.z, brushSize());
      setNotifications((existing) => [...existing, `Bulldozed for ${cost}`]);
    }
    if (tool === 'terrain') {
      terrainTool.sculpt(tileId, 0.2);
    }
    if (tool === 'services') {
      servicesSystem.placeService(tileId, selectedService());
      setNotifications((existing) => [...existing, `${selectedService()} service placed`]);
    }
    if (tool === 'road') {
      if (!pendingRoadStart) {
        pendingRoadStart = tileId;
        roadTool.begin(tileId);
      } else {
        const placed = roadTool.commit(tileId);
        pendingRoadStart = null;
        if (placed > 0) {
          setNotifications((existing) => [...existing, `Road segments added: ${placed}`]);
        }
      }
    }

    setCity(grid.getState());
    demandSystem.update(0);
  };

  let lastFrame = performance.now();
  let economyElapsed = 0;
  let trafficElapsed = 0;
  let citizensElapsed = 0;

  engine.runRenderLoop(async () => {
    const now = performance.now();
    const dt = (now - lastFrame) * Math.max(0, simulationSpeed());
    lastFrame = now;

    if (simulationSpeed() === 0) {
      scene.render();
      return;
    }

    sceneSystem.update(dt);
    terrainSystem.update(dt);
    rendererSystem.update(dt);
    postFxSystem.update(dt);
    demandSystem.update(dt);
    budgetSystem.update(dt);
    servicesSystem.update(dt);

    let nextCity = servicesSystem.getState();
    nextCity = demandSystem.compute(nextCity);
    setCity(nextCity);
    grid.setState(nextCity);

    economyElapsed += dt;
    trafficElapsed += dt;
    citizensElapsed += dt;

    if (economyElapsed >= 5000) {
      economyElapsed = 0;
      const message = await economyWorker.tick({ type: 'ECONOMY_TICK', payload: { budget: city().budget, city: city() } });
      if (message.type === 'BANKRUPTCY_WARNING') {
        setNotifications((existing) => [...existing, 'Bankruptcy warning']);
      }
      if (message.type === 'ECONOMY_RESULT' && message.payload) {
        const next = { ...city(), budget: message.payload.budget };
        setCity(next);
        grid.setState(next);
      }
    }

    if (trafficElapsed >= 200) {
      trafficElapsed = 0;
      await trafficWorker.tick({ type: 'TRAFFIC_TICK', payload: { adjacency: grid.getRoadGraphAdjacency(), citizenCounts: {} } });
    }

    if (citizensElapsed >= 500) {
      citizensElapsed = 0;
      await citizensWorker.tick({ type: 'CITIZENS_TICK', payload: { city: city(), gameTime: city().gameTime } });
    }

    if (roadChangesSinceLastSave > ROAD_AUTOSAVE_SEGMENT_DELTA) {
      void persist('autosave');
    }

    scene.render();
  });

  window.addEventListener('beforeunload', () => {
    void saveAdapter.save('autosave', city());
  });

  window.addEventListener('resize', () => engine.resize());

  const root = document.getElementById('root');
  if (!root) {
    return;
  }
  root.style.position = 'relative';
  root.style.zIndex = '1';

  render(
    () =>
      App({
        city: city(),
        demand: city().demand,
        budget: city().budget,
        selectedTile: selectedTileId() ? city().tiles[selectedTileId() as string] ?? null : null,
        selectedBuilding: selectedTileId()
          ? (() => {
              const tile = city().tiles[selectedTileId() as string];
              if (!tile?.buildingId) {
                return null;
              }
              return city().buildings[tile.buildingId] ?? null;
            })()
          : null,
        activeTool: activeTool(),
        selectedZone: selectedZone(),
        selectedService: selectedService(),
        brushSize: brushSize(),
        notifications: notifications(),
        saveState: saveState(),
        graphics: graphics(),
        simulationSpeed: simulationSpeed(),
        audioVolume: audioVolume(),
        onToolChange: (tool) => setActiveTool(tool),
        onZoneChange: (zone) => setSelectedZone(zone),
        onServiceChange: (service) => setSelectedService(service),
        onBrushSizeChange: (size) => setBrushSize(size),
        onDemolish: () => {
          const selected = selectedTileId();
          if (!selected) {
            return;
          }
          const [x, z] = selected.split('_').map(Number);
          grid.demolish(x, z);
          setCity(grid.getState());
        },
        onPanTo: (mapX, mapZ) => {
          const tileX = (mapX / 200) * 150;
          const tileZ = (mapZ / 200) * 150;
          const worldX = tileX * 10 - (150 * 10) / 2;
          const worldZ = tileZ * 10 - (150 * 10) / 2;
          sceneSystem.panToWorld(worldX, worldZ);
        },
        onTaxRateChange: (zone, value) => {
          budgetSystem.setTaxRate(zone, value);
          const next = budgetSystem.getState();
          setCity(next);
          grid.setState(next);
        },
        onBorrow: () => {
          budgetSystem.borrowLoan();
          const next = budgetSystem.getState();
          setCity(next);
          grid.setState(next);
        },
        onGraphicsChange: (key, enabled) => {
          setGraphics((prev) => ({ ...prev, [key]: enabled }));
          if (key === 'ssao') {
            postFxSystem.setSSAOEnabled(enabled);
          }
          if (key === 'bloom') {
            postFxSystem.setBloomEnabled(enabled);
          }
          if (key === 'shadows') {
            postFxSystem.setShadowsEnabled(enabled);
          }
          if (key === 'dof') {
            postFxSystem.setDofEnabled(enabled);
          }
        },
        onSimulationSpeedChange: (speed) => setSimulationSpeed(speed),
        onAudioVolumeChange: (value) => setAudioVolume(value),
        onManualSave: () => {
          void persist('manual');
        }
      }),
    root
  );
};

