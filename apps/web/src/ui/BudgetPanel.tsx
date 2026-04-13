import type { Component } from 'solid-js';
import { createSignal, Show } from 'solid-js';

import type { BudgetState } from '../types';

interface BudgetPanelProps {
  budget: BudgetState;
  onTaxRateChange: (zone: 'residential' | 'commercial' | 'industrial', value: number) => void;
  onBorrow: () => void;
}

export const BudgetPanel: Component<BudgetPanelProps> = (props) => {
  const [expanded, setExpanded] = createSignal(false);

  return (
	<section style={{ display: 'grid', gap: '6px' }}>
	  <strong style={{ color: props.budget.balance >= 0 ? '#82e6a1' : '#f38787' }}>Balance: {Math.round(props.budget.balance)}</strong>
	  <span>Income: {Math.round(props.budget.income)}</span>
	  <span>Expenses: {Math.round(props.budget.expenses)}</span>
	  <button type="button" onClick={() => setExpanded(!expanded())}>
		{expanded() ? 'Hide' : 'Expand'}
	  </button>
	  <Show when={expanded()}>
		{(['residential', 'commercial', 'industrial'] as const).map((zone) => (
		  <label style={{ display: 'grid', gap: '4px' }}>
			{zone}
			<input
			  type="range"
			  min="0"
			  max="0.2"
			  step="0.01"
			  value={props.budget.taxRates[zone]}
			  onInput={(event) => props.onTaxRateChange(zone, Number(event.currentTarget.value))}
			/>
		  </label>
		))}
		<button
		  type="button"
		  onClick={() => {
			if (window.confirm('Borrow a loan increment?')) {
			  props.onBorrow();
			}
		  }}
		>
		  Borrow
		</button>
	  </Show>
	</section>
  );
};
