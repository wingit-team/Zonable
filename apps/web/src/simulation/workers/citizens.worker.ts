/// <reference lib="webworker" />

import type { CitizenAgent, CityState, WorkerMessage } from '../../types';

type CitizensTickPayload = {
  city: CityState;
  gameTime: number;
};

type CitizenPosition = { id: string; progress: number; state: CitizenAgent['state'] };

type CitizensResultPayload = {
  positions: CitizenPosition[];
  happinessByTile: Record<string, number>;
};

const MAX_AGENTS = 5000;
let agents: CitizenAgent[] = [];

const nextState = (state: CitizenAgent['state'], gameHour: number): CitizenAgent['state'] => {
  if (gameHour >= 7 && gameHour < 9 && state === 'home') {
    return 'commuting';
  }
  if (gameHour >= 9 && gameHour < 17 && state === 'commuting') {
    return 'working';
  }
  if (gameHour >= 17 && gameHour < 19 && state === 'working') {
    return 'returning';
  }
  if ((gameHour >= 19 || gameHour < 7) && state === 'returning') {
    return 'home';
  }
  return state;
};

self.onmessage = (event: MessageEvent<WorkerMessage<CitizensTickPayload>>): void => {
  if (event.data.type !== 'CITIZENS_TICK') {
    return;
  }

  const { city, gameTime } = event.data.payload;
  const gameHour = Math.floor((gameTime / 1000) % 24);

  if (agents.length === 0) {
    const residential = Object.values(city.buildings).filter((building) => building.type === 'residential');
    agents = residential.slice(0, MAX_AGENTS).map((building, i) => ({
      id: `citizen_${i}`,
      homeTileId: building.tileId,
      workTileId: null,
      state: 'home',
      pathProgress: 0,
      happiness: 0.8
    }));
  }

  agents = agents.map((agent) => {
    const state = nextState(agent.state, gameHour);
    const pathProgress = state === 'commuting' || state === 'returning' ? Math.min(1, agent.pathProgress + 0.15) : 0;
    const noPath = state === 'commuting' && !agent.workTileId;
    return {
      ...agent,
      state,
      pathProgress,
      happiness: Math.max(0, Math.min(1, agent.happiness + (noPath ? -0.1 : 0.005)))
    };
  });

  const positions = agents.map((agent) => ({ id: agent.id, progress: agent.pathProgress, state: agent.state }));
  const happinessByTile: Record<string, number> = {};
  agents.forEach((agent) => {
    const previous = happinessByTile[agent.homeTileId] ?? agent.happiness;
    happinessByTile[agent.homeTileId] = (previous + agent.happiness) / 2;
  });

  const message: WorkerMessage<CitizensResultPayload> = {
    type: 'CITIZENS_RESULT',
    payload: { positions, happinessByTile }
  };
  self.postMessage(message);
};

export {};
