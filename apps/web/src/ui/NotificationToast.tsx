import type { Component } from 'solid-js';
import { createMemo, createSignal, onCleanup, onMount } from 'solid-js';

interface NotificationToastProps {
  messages: string[];
}

export const NotificationToast: Component<NotificationToastProps> = (props) => {
  const [index, setIndex] = createSignal(0);
  const message = createMemo(() => props.messages[index()] ?? null);

  onMount(() => {
	const intervalId = window.setInterval(() => {
	  if (props.messages.length === 0) {
		return;
	  }
	  setIndex((current) => (current + 1) % props.messages.length);
	}, 4000);

	onCleanup(() => window.clearInterval(intervalId));
  });

  return (
	<section style={{ transform: 'translateY(0)', transition: 'transform 180ms ease' }}>
	  {message()}
	</section>
  );
};
