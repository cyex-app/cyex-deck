// One shared 1 s clock (1.21 live uptime). The interval runs only while an effect or
// template reads `nowSecond()`, and stops when the last reader is gone.
import { createSubscriber } from 'svelte/reactivity';

import { everyShown } from './awake';

// V107-H1 Not while the window is hidden: once when it shows.
const subscribe = createSubscriber((update) => everyShown(update, 1000));

/** Date.now(), re-read every second by whatever reactive scope calls it. */
export function nowSecond(): number {
  subscribe();
  return Date.now();
}
