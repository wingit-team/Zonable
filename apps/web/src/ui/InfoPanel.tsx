import type { Component } from 'solid-js';

import { getServiceCoverageForTile, type ServiceCoverageState } from '../simulation/service.coverage';
import type { Building, CityState, Tile } from '../types';

interface InfoPanelProps {
  city: CityState;
  tile: Tile | null;
  building: Building | null;
  onDemolish: () => void;
}

const boolLabel = (value: boolean): string => (value ? 'yes' : 'no');

export const InfoPanel: Component<InfoPanelProps> = (props) => {
  const coverage: ServiceCoverageState | null = props.tile ? getServiceCoverageForTile(props.city, props.tile.id) : null;

  return (
	<section style={{ display: 'grid', gap: '4px', 'min-width': '210px' }}>
	  <strong>Tile Info</strong>
	  <span>Zone: {props.tile?.zone ?? 'n/a'}</span>
	  <span>Land value: {props.tile ? props.tile.landValue.toFixed(2) : 'n/a'}</span>
	  <span>Building level: {props.building?.level ?? 'n/a'}</span>
	  <span>Population/jobs: {props.building?.population ?? 0}</span>
	  <span>Happiness: {props.building ? props.building.happiness.toFixed(2) : 'n/a'}</span>
	  <span>Fire: {coverage ? boolLabel(coverage.fire) : 'n/a'}</span>
	  <span>Police: {coverage ? boolLabel(coverage.police) : 'n/a'}</span>
	  <span>Health: {coverage ? boolLabel(coverage.health) : 'n/a'}</span>
	  <span>Education: {coverage ? boolLabel(coverage.education) : 'n/a'}</span>
	  <span>Power: {coverage ? boolLabel(coverage.power) : 'n/a'}</span>
	  <span>Water: {coverage ? boolLabel(coverage.water) : 'n/a'}</span>
	  {props.building && (
		<button type="button" onClick={props.onDemolish}>
		  Demolish
		</button>
	  )}
	</section>
  );
};
