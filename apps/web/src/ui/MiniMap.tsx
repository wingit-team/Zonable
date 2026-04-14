import type { Component } from 'solid-js';
import { createEffect } from 'solid-js';

import type { CityState } from '../types';

interface MiniMapProps {
  city: CityState;
  camera: { x: number; z: number; radius: number };
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

  createEffect(() => {
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
	ctx.clearRect(0, 0, size, size);

	Object.values(props.city.tiles).forEach((tile) => {
	  ctx.fillStyle = zoneColor(tile.zone);
	  ctx.fillRect(tile.x * pixelSize, tile.z * pixelSize, pixelSize, pixelSize);
	});

	const cameraTileX = (props.camera.x + (gridSize * 10) / 2) / 10;
	const cameraTileZ = (props.camera.z + (gridSize * 10) / 2) / 10;
	const viewportTiles = Math.max(8, props.camera.radius * 0.45);
	const viewportPixels = viewportTiles * pixelSize;
	ctx.strokeStyle = '#ffffff';
	ctx.lineWidth = 1;
	ctx.strokeRect(cameraTileX * pixelSize - viewportPixels / 2, cameraTileZ * pixelSize - viewportPixels / 2, viewportPixels, viewportPixels);
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
