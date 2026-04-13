import type { Component } from 'solid-js';
import { onMount } from 'solid-js';

import type { CityState } from '../types';

interface MiniMapProps {
  city: CityState;
  onPanTo: (x: number, z: number) => void;
}

const zoneColor = (zone: string): string => {
  if (zone === 'residential') {
	return '#7ec87e';
  }
  if (zone === 'commercial') {
	return '#7eafc8';
  }
  if (zone === 'industrial') {
	return '#c8c07e';
  }
  return '#31463b';
};

export const MiniMap: Component<MiniMapProps> = (props) => {
  let canvasRef: HTMLCanvasElement | undefined;

  onMount(() => {
	if (!canvasRef) {
	  return;
	}
	const ctx = canvasRef.getContext('2d');
	if (!ctx) {
	  return;
	}

	const size = 200;
	const gridSize = Math.sqrt(Object.keys(props.city.tiles).length);
	const pixelSize = size / gridSize;
	Object.values(props.city.tiles).forEach((tile) => {
	  ctx.fillStyle = zoneColor(tile.zone);
	  ctx.fillRect(tile.x * pixelSize, tile.z * pixelSize, pixelSize, pixelSize);
	});
  });

  return (
	<canvas
	  ref={canvasRef}
	  width={200}
	  height={200}
	  onClick={(event) => {
		const rect = event.currentTarget.getBoundingClientRect();
		props.onPanTo(event.clientX - rect.left, event.clientY - rect.top);
	  }}
	/>
  );
};
