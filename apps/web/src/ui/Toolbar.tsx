import type { Component } from 'solid-js';

import type { ServiceType, ZoneType } from '../types';

export type ToolName = 'road' | 'zone' | 'bulldoze' | 'terrain' | 'services';

interface ToolbarProps {
  activeTool: ToolName;
  brushSize: number;
  selectedZone: Exclude<ZoneType, 'none'>;
  selectedService: ServiceType;
  onToolChange: (tool: ToolName) => void;
  onBrushSizeChange: (size: number) => void;
  onZoneChange: (zone: Exclude<ZoneType, 'none'>) => void;
  onServiceChange: (service: ServiceType) => void;
}

const buttonStyle = (active: boolean): Record<string, string> => ({
  padding: '6px 10px',
  border: active ? '1px solid #9cc9ff' : '1px solid #42516a',
  background: active ? 'rgba(79, 125, 193, 0.45)' : 'rgba(11, 18, 31, 0.75)',
  color: '#e6eefc',
  cursor: 'pointer'
});

export const Toolbar: Component<ToolbarProps> = (props) => (
  <aside style={{ display: 'flex', gap: '8px', 'align-items': 'center' }}>
	{(['road', 'zone', 'bulldoze', 'terrain', 'services'] as const).map((tool) => (
	  <button type="button" style={buttonStyle(props.activeTool === tool)} onClick={() => props.onToolChange(tool)}>
		{tool}
	  </button>
	))}
	{props.activeTool === 'zone' && (
	  <div style={{ display: 'flex', gap: '4px' }}>
		{(['residential', 'commercial', 'industrial'] as const).map((zone) => (
		  <button type="button" style={buttonStyle(props.selectedZone === zone)} onClick={() => props.onZoneChange(zone)} title={zone}>
			{zone === 'residential' ? 'Res' : zone === 'commercial' ? 'Com' : 'Ind'}
		  </button>
		))}
	  </div>
	)}
	{props.activeTool === 'services' && (
	  <div style={{ display: 'flex', gap: '4px' }}>
		{(['fire', 'police', 'health', 'education', 'power', 'water'] as const).map((service) => (
		  <button type="button" style={buttonStyle(props.selectedService === service)} onClick={() => props.onServiceChange(service)}>
			{service[0].toUpperCase()}
		  </button>
		))}
	  </div>
	)}
	{props.activeTool === 'terrain' && (
	  <label style={{ display: 'flex', gap: '4px', 'align-items': 'center' }}>
		Terrain Brush
		<select value={props.brushSize} onInput={(event) => props.onBrushSizeChange(Number(event.currentTarget.value))}>
		  {[1, 2, 3, 4, 5].map((size) => (
			<option value={size}>{size}</option>
		  ))}
		</select>
	  </label>
	)}
  </aside>
);
