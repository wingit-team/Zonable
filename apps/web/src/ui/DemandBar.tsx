import type { Component } from 'solid-js';

import type { DemandState } from '../types';

interface DemandBarProps {
  demand: DemandState;
}

const barStyle = (color: string, value: number): Record<string, string> => ({
  width: '16px',
  height: `${Math.round(value * 80)}px`,
  background: color,
  transition: 'height 220ms ease'
});

export const DemandBar: Component<DemandBarProps> = (props) => (
  <section style={{ display: 'flex', gap: '8px', 'align-items': 'flex-end' }} title="R rises with housing shortages; C rises with low C:R ratio; I falls with polluted residential adjacency.">
	<div style={barStyle('#7ec87e', props.demand.residential)} />
	<div style={barStyle('#7eafc8', props.demand.commercial)} />
	<div style={barStyle('#c8c07e', props.demand.industrial)} />
  </section>
);
