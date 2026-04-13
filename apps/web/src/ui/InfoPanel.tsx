import type { Component } from 'solid-js';

import type { Building, Tile } from '../types';

interface InfoPanelProps {
  tile: Tile | null;
  building: Building | null;
  onDemolish: () => void;
}

export const InfoPanel: Component<InfoPanelProps> = (props) => (
  <section style={{ display: 'grid', gap: '4px', 'min-width': '210px' }}>
	<strong>Tile Info</strong>
	<span>Zone: {props.tile?.zone ?? 'n/a'}</span>
	<span>Land value: {props.tile ? props.tile.landValue.toFixed(2) : 'n/a'}</span>
	<span>Building level: {props.building?.level ?? 'n/a'}</span>
	<span>Population/jobs: {props.building?.population ?? 0}</span>
	<span>Happiness: {props.building ? props.building.happiness.toFixed(2) : 'n/a'}</span>
	<span>Coverage: {props.tile?.serviceIds.join(', ') || 'none'}</span>
	{props.building && (
	  <button type="button" onClick={props.onDemolish}>
		Demolish
	  </button>
	)}
  </section>
);
