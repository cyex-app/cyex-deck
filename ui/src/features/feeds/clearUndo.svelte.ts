// V107-MF Clear with Undo for every feed tab. The rows hide at once; the feed is cleared for
// real once the toast's time is up (or the tab closes). Only the rows that were hidden go,
// so rows that arrived in the meantime stay (V107-N8a).

import { errorText } from '../../lib/errors.ts';
import { ACTION_MS, toast } from '../../lib/toast.svelte.ts';

export class ClearUndo {
  hidden = $state.raw<Set<string> | null>(null);
  #timer: ReturnType<typeof setTimeout> | undefined;
  #toast: number | undefined;
  #keys: () => string[];
  #commit: (keys: string[]) => Promise<void>;
  #done: (gone: Set<string>) => void;

  /** `keys`: the keys of the rows now. `commit`: really clears those keys. `done`: drops them locally. */
  constructor(keys: () => string[], commit: (keys: string[]) => Promise<void>, done: (gone: Set<string>) => void) {
    this.#keys = keys;
    this.#commit = commit;
    this.#done = done;
  }

  /** Whether the row stays visible. */
  shows = (key: string) => !this.hidden?.has(key);

  clear = () => {
    this.flush();
    const keys = this.#keys();
    if (keys.length === 0) return;
    this.hidden = new Set(keys);
    this.#timer = setTimeout(this.flush, ACTION_MS);
    this.#toast = toast.info('Feed cleared.', { action: { label: 'Undo', run: this.undo } });
  };

  undo = () => {
    clearTimeout(this.#timer);
    this.#toast = undefined;
    this.hidden = null;
  };

  flush = () => {
    clearTimeout(this.#timer);
    if (this.#toast !== undefined) toast.dismiss(this.#toast);
    this.#toast = undefined;
    const gone = this.hidden;
    if (!gone) return;
    this.hidden = null;
    this.#done(gone);
    this.#commit([...gone]).catch((e) => toast.error(errorText(e)));
  };
}
