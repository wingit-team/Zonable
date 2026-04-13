import type { Component } from 'solid-js';

export type ToolName = 'road' | 'zone' | 'bulldoze' | 'terrain' | 'services';

interface ToolbarProps {
  activeTool: ToolName;
  brushSize: number;
  onToolChange: (tool: ToolName) => void;
  onBrushSizeChange: (size: number) => void;
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
	{(props.activeTool === 'zone' || props.activeTool === 'bulldoze') && (
	  <label style={{ display: 'flex', gap: '4px', 'align-items': 'center' }}>
		Brush
		<select value={props.brushSize} onInput={(event) => props.onBrushSizeChange(Number(event.currentTarget.value))}>
		  {[1, 2, 3, 4, 5].map((size) => (
			<option value={size}>{size}</option>
		  ))}
		</select>
	  </label>
	)}
  </aside>
);
