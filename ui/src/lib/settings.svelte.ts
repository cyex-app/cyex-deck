// Reactive copy of the Rust settings store. Rust is the source of truth: every write
// goes through `patch`, and changes from other windows arrive as events.

import { settingsIpc, type DeepPartial, type SettingsValues } from '../ipc/settings';
import type { Partition } from '../ipc/bindings/Partition';
import { InFlight, mergePatch } from './mergePatch';
import { toast } from './toast.svelte';

class SettingsStore {
  v = $state({} as SettingsValues);
  /** Edits sent and not answered yet: until then the store shows them already. */
  private flight = new InFlight<Partition>();

  async load(): Promise<void> {
    this.v = await settingsIpc.all();
    await settingsIpc.onChanged((partition, value) => {
      // Our own edits come back as answers; an echo from the middle would undo a newer one.
      if (this.flight.busy(partition)) return;
      (this.v as Record<Partition, unknown>)[partition] = value;
    });
    const reset = await settingsIpc.takeLoadReport();
    if (reset.length) {
      toast.error(`Couldn't read ${reset.join(', ')} settings, so they were reset. The old file is kept next to it.`);
    }
  }

  /** Shows the edit at once, so the next one builds on it (V104-04: a word typed into a
   * list and a box ticked right after both started from the old list); Rust's answer to the
   * newest edit then replaces it with what was really saved. */
  async patch<P extends Partition>(partition: P, patch: DeepPartial<SettingsValues[P]>): Promise<void> {
    const n = this.flight.start(partition);
    this.v[partition] = mergePatch($state.snapshot(this.v[partition]) as SettingsValues[P], patch);
    let value: SettingsValues[P] | undefined;
    try {
      value = await settingsIpc.patch(partition, patch);
    } catch (e) {
      toast.error(String(e));
    }
    if (!this.flight.settle(partition, n)) return;
    this.v[partition] = value ?? (await settingsIpc.all())[partition];
  }

  async reset<P extends Partition>(partition: P): Promise<void> {
    this.v[partition] = await settingsIpc.reset(partition);
  }
}

export const settings = new SettingsStore();
