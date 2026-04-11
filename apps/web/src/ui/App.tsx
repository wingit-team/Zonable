import type { Component } from 'solid-js';
import { BudgetPanel } from './BudgetPanel';
import { DemandBar } from './DemandBar';
import { InfoPanel } from './InfoPanel';
import { MiniMap } from './MiniMap';
import { NotificationToast } from './NotificationToast';
import { SettingsModal } from './SettingsModal';
import { Toolbar } from './Toolbar';

const panelStyle = {
  position: 'absolute',
  color: '#d6deeb',
  padding: '8px 10px',
  background: 'rgba(4, 8, 16, 0.68)',
  border: '1px solid rgba(90, 108, 140, 0.4)',
  'font-family': 'Inter, system-ui, sans-serif',
  'font-size': '12px'
} as const;

export const App: Component = () => (
  <div>
    <div style={{ ...panelStyle, top: '10px', left: '10px' }}>
      <Toolbar />
    </div>
    <div style={{ ...panelStyle, top: '10px', right: '10px' }}>
      <DemandBar />
      <BudgetPanel />
    </div>
    <div style={{ ...panelStyle, bottom: '10px', left: '10px' }}>
      <InfoPanel />
      <MiniMap />
    </div>
    <div style={{ ...panelStyle, bottom: '10px', right: '10px' }}>
      <NotificationToast />
      <SettingsModal />
    </div>
  </div>
);
