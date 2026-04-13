import type { Component } from 'solid-js';
import { createSignal, onCleanup, onMount, Show } from 'solid-js';

interface SettingsModalProps {
  graphics: {
	ssao: boolean;
	bloom: boolean;
	shadows: boolean;
	dof: boolean;
  };
  simulationSpeed: 0 | 1 | 2 | 3;
  audioVolume: number;
  onGraphicsChange: (key: 'ssao' | 'bloom' | 'shadows' | 'dof', enabled: boolean) => void;
  onSimulationSpeedChange: (speed: 0 | 1 | 2 | 3) => void;
  onAudioVolumeChange: (value: number) => void;
  onManualSave: () => void;
}

export const SettingsModal: Component<SettingsModalProps> = (props) => {
  const [open, setOpen] = createSignal(false);
  onMount(() => {
	const onKeydown = (event: KeyboardEvent): void => {
	  if (event.key === 'Escape') {
		setOpen((value) => !value);
	  }
	};
	window.addEventListener('keydown', onKeydown);
	onCleanup(() => window.removeEventListener('keydown', onKeydown));
  });

  return (
	<section>
	  <button type="button" onClick={() => setOpen(!open())}>
		Settings
	  </button>
	  <Show when={open()}>
		<div style={{ display: 'grid', gap: '6px', padding: '8px', border: '1px solid #42516a' }}>
		  {(['ssao', 'bloom', 'shadows', 'dof'] as const).map((key) => (
			<label>
			  <input type="checkbox" checked={props.graphics[key]} onInput={(event) => props.onGraphicsChange(key, event.currentTarget.checked)} />
			  {key.toUpperCase()}
			</label>
		  ))}
		  <label>
			Simulation speed
			<select value={props.simulationSpeed} onInput={(event) => props.onSimulationSpeedChange(Number(event.currentTarget.value) as 0 | 1 | 2 | 3)}>
			  <option value={0}>paused</option>
			  <option value={1}>1x</option>
			  <option value={2}>2x</option>
			  <option value={3}>3x</option>
			</select>
		  </label>
		  <label>
			Audio
			<input type="range" min="0" max="1" step="0.01" value={props.audioVolume} onInput={(event) => props.onAudioVolumeChange(Number(event.currentTarget.value))} />
		  </label>
		  <button type="button" onClick={props.onManualSave}>
			Save
		  </button>
		</div>
	  </Show>
	</section>
  );
};
