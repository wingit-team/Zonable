import type { Component } from 'solid-js';
import { Show } from 'solid-js';

import type { Building, BudgetState, CityState, DemandState, ServiceType, Tile, ZoneType } from '../types';
import { BudgetPanel } from './BudgetPanel';
import { DemandBar } from './DemandBar';
import { InfoPanel } from './InfoPanel';
import { MiniMap } from './MiniMap';
import { NotificationToast } from './NotificationToast';
import { SettingsModal } from './SettingsModal';
import { Toolbar, type ToolName } from './Toolbar';

interface AppProps {
  city: CityState;
  demand: DemandState;
  budget: BudgetState;
  selectedTile: Tile | null;
  selectedBuilding: Building | null;
  activeTool: ToolName;
  selectedZone: Exclude<ZoneType, 'none'>;
  selectedService: ServiceType;
  brushSize: number;
  notifications: string[];
  saveState: 'idle' | 'saving' | 'saved';
  graphics: { ssao: boolean; bloom: boolean; shadows: boolean; dof: boolean };
  simulationSpeed: 0 | 1 | 2 | 3;
  audioVolume: number;
  onToolChange: (tool: ToolName) => void;
  onZoneChange: (zone: Exclude<ZoneType, 'none'>) => void;
  onServiceChange: (service: ServiceType) => void;
  onBrushSizeChange: (size: number) => void;
  onDemolish: () => void;
  onPanTo: (x: number, z: number) => void;
  onTaxRateChange: (zone: 'residential' | 'commercial' | 'industrial', value: number) => void;
  onBorrow: () => void;
  onGraphicsChange: (key: 'ssao' | 'bloom' | 'shadows' | 'dof', enabled: boolean) => void;
  onSimulationSpeedChange: (speed: 0 | 1 | 2 | 3) => void;
  onAudioVolumeChange: (value: number) => void;
  onManualSave: () => void;
}

const panelStyle = {
  position: 'absolute',
  color: '#d6deeb',
  padding: '8px 10px',
  background: 'rgba(4, 8, 16, 0.68)',
  border: '1px solid rgba(90, 108, 140, 0.4)',
  'font-family': 'Inter, system-ui, sans-serif',
  'font-size': '12px'
} as const;

export const App: Component<AppProps> = (props) => (
  <div>
    <style>
      {`@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
        @keyframes pop { from { transform: scale(0.9); opacity: 0; } to { transform: scale(1); opacity: 1; } }`}
    </style>
    <div style={{ ...panelStyle, bottom: '10px', left: '50%', transform: 'translateX(-50%)' }}>
      <Toolbar
        activeTool={props.activeTool}
        selectedZone={props.selectedZone}
        selectedService={props.selectedService}
        brushSize={props.brushSize}
        onToolChange={props.onToolChange}
        onZoneChange={props.onZoneChange}
        onServiceChange={props.onServiceChange}
        onBrushSizeChange={props.onBrushSizeChange}
      />
    </div>
    <div style={{ ...panelStyle, top: '10px', right: '10px', display: 'grid', gap: '8px' }}>
      <div style={{ display: 'flex', 'justify-content': 'space-between', 'align-items': 'center' }}>
        <DemandBar demand={props.demand} />
        <Show when={props.saveState === 'saving'} fallback={<span style={props.saveState === 'saved' ? { animation: 'pop 180ms ease' } : {}}>{props.saveState === 'saved' ? 'Saved ✓' : ''}</span>}>
          <span style={{ animation: 'spin 1s linear infinite' }}>Saving...</span>
        </Show>
      </div>
      <BudgetPanel budget={props.budget} onTaxRateChange={props.onTaxRateChange} onBorrow={props.onBorrow} />
    </div>
    <div style={{ ...panelStyle, bottom: '10px', right: '10px', display: 'grid', gap: '8px' }}>
      <InfoPanel tile={props.selectedTile} building={props.selectedBuilding} onDemolish={props.onDemolish} />
      <MiniMap city={props.city} onPanTo={props.onPanTo} />
    </div>
    <div style={{ ...panelStyle, top: '10px', left: '10px', display: 'grid', gap: '8px' }}>
      <NotificationToast messages={props.notifications} />
      <SettingsModal
        graphics={props.graphics}
        simulationSpeed={props.simulationSpeed}
        audioVolume={props.audioVolume}
        onGraphicsChange={props.onGraphicsChange}
        onSimulationSpeedChange={props.onSimulationSpeedChange}
        onAudioVolumeChange={props.onAudioVolumeChange}
        onManualSave={props.onManualSave}
      />
    </div>
  </div>
);
