// One shared 1 s clock (1.21 live uptime). The interval runs only while an effect or
// template reads `nowSecond()`, and stops when the last reader is gone.
import { createSubscriber } from 'svelte/reactivity';

const subscribe = createSubscriber((update) => {
  const timer = setInterval(update, 1000);
  return () => clearInterval(timer);
});

/** Date.now(), re-read every second by whatever reactive scope calls it. */
export function nowSecond(): number {
  subscribe();
  return Date.now();
}
