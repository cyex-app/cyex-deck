// Reactive copy of the Rust settings store. Rust is the source of truth: every write
// goes through `patch`, and changes from other windows arrive as events.

import { settingsIpc, type DeepPartial, type SettingsValues } from '../ipc/settings';
import type { Partition } from '../ipc/bindings/Partition';
import { toast } from './toast.svelte';

class SettingsStore {
  v = $state({} as SettingsValues);

  async load(): Promise<void> {
    this.v = await settingsIpc.all();
    await settingsIpc.onChanged((partition, value) => {
      (this.v as Record<Partition, unknown>)[partition] = value;
    });
    const reset = await settingsIpc.takeLoadReport();
    if (reset.length) {
      toast.error(`Couldn't read ${reset.join(', ')} settings, so they were reset. The old file is kept next to it.`);
    }
  }

  async patch<P extends Partition>(partition: P, patch: DeepPartial<SettingsValues[P]>): Promise<void> {
    try {
      this.v[partition] = await settingsIpc.patch(partition, patch);
    } catch (e) {
      toast.error(String(e));
    }
  }

  async reset<P extends Partition>(partition: P): Promise<void> {
    this.v[partition] = await settingsIpc.reset(partition);
  }
}

export const settings = new SettingsStore();
