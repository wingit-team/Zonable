/// <reference lib="webworker" />

import type { WorkerMessage } from '../../types';

type TrafficTickPayload = {
  adjacency: Record<string, string[]>;
  citizenCounts: Record<string, number>;
};

type TrafficResultPayload = Record<string, number>;

const congestionState: Record<string, number> = {};
const SEGMENT_CAPACITY = 30;

const clamp01 = (value: number): number => Math.max(0, Math.min(1, value));

self.onmessage = (event: MessageEvent<WorkerMessage<TrafficTickPayload>>): void => {
  if (event.data.type !== 'TRAFFIC_TICK') {
    return;
  }

  const result: TrafficResultPayload = {};
  Object.keys(event.data.payload.adjacency).forEach((segmentId) => {
    const count = event.data.payload.citizenCounts[segmentId] ?? 0;
    const previous = congestionState[segmentId] ?? 0;
    const pressure = count > SEGMENT_CAPACITY ? 0.08 : -0.05;
    const next = clamp01(previous + pressure);
    congestionState[segmentId] = next;
    result[segmentId] = next;
  });

  const message: WorkerMessage<TrafficResultPayload> = { type: 'TRAFFIC_RESULT', payload: result };
  self.postMessage(message);
};

export {};
