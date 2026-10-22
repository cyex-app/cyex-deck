// Reactive copy of the Rust settings store. Rust is the source of truth: every write
// goes through `patch`, and changes from other windows arrive as events.

import { settingsIpc, type DeepPartial, type SettingsValues } from '../ipc/settings';
import type { Partition } from '../ipc/bindings/Partition';
import { InFlight, mergePatch, sameJson, type Settled } from './mergePatch';
import { partitionList } from './partitions';
import { errorText } from './errors';
import { toast } from './toast.svelte';

class SettingsStore {
  v = $state({} as SettingsValues);
  /** Edits sent and not answered yet: until then the store shows them already. */
  private flight = new InFlight<Partition>();

  async load(): Promise<void> {
    this.v = await settingsIpc.all();
    await settingsIpc.onChanged((partition, value) => {
      // Our own edits come back as answers; an echo from the middle would undo a newer one.
      // V107-A8 It may also be another window's change, so it's checked once the answer lands.
      if (this.flight.busy(partition)) {
        this.flight.miss(partition, value);
        return;
      }
      (this.v as Record<Partition, unknown>)[partition] = value;
    });
    const { reset, restored, restoredFrom } = await settingsIpc.takeLoadReport();
    if (reset.length) {
      // t5 Names as in Backup and sharing; a file can also be only partly reset now.
      toast.error(`Some settings couldn’t be read and went back to default: ${partitionList(reset)}. The old files are kept in the settings folder.`);
    }
    if (restored.length) {
      // V107-N10a Cut short or empty: the newest backup that still reads.
      toast.error(`Some settings couldn’t be read and came back from ${restoredFrom ? `the backup of ${restoredFrom}` : 'a backup'}: ${partitionList(restored)}.`);
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
      toast.error(errorText(e, 'save the settings'));
    }
    await this.finish(partition, n, value);
  }

  /** V107-A8 An edit sent by other code (tabs): pass the number to `finish`. Change events
   * for `partition` wait until then. */
  begin(partition: Partition): number {
    return this.flight.start(partition);
  }

  /** V107-A8 Takes the answer to edit `n` if it is the newest one. When a change event was
   * skipped meanwhile and differs from it (another window, Rust itself), the partition is
   * read again. `value` undefined: the edit failed; read it again if `readIfFailed`. */
  async finish<P extends Partition>(
    partition: P,
    n: number,
    value: SettingsValues[P] | undefined,
    readIfFailed = true,
  ): Promise<void> {
    const done = this.flight.settle(partition, n);
    if (done) await this.land(partition, done, value, readIfFailed);
  }

  private async land<P extends Partition>(
    partition: P,
    done: Settled,
    value: SettingsValues[P] | undefined,
    readIfNone: boolean,
  ): Promise<void> {
    // Bounded: a window that keeps changing can't keep this one reading.
    for (let tries = 0; ; tries++) {
      const stale = done.missed !== undefined && !sameJson(done.missed.value, value);
      if (value !== undefined && !stale) {
        this.v[partition] = value;
        return;
      }
      if (value === undefined && !stale && !readIfNone) return;
      if (tries === 3) {
        if (value !== undefined) this.v[partition] = value;
        return;
      }
      const n = this.flight.start(partition);
      value = undefined;
      try {
        value = (await settingsIpc.all())[partition];
      } catch (e) {
        toast.error(errorText(e, 'read the settings'));
      }
      const next = this.flight.settle(partition, n);
      if (!next) return;
      done = next;
      readIfNone = false;
    }
  }

  /** V105-12 Empty rules go once the page is left. Counts as the newest edit of each part, so
   * an edit still on its way (a field that saved as the page changed) can't bring them back. */
  async pruneBlank(): Promise<void> {
    const parts = ['highlights', 'commands'] as const;
    const started = parts.map((p) => [p, this.flight.start(p)] as const);
    let values: Partial<SettingsValues> = {};
    try {
      values = await settingsIpc.pruneBlank();
    } catch (e) {
      toast.error(errorText(e, 'save the settings'));
    }
    for (const [p, n] of started) {
      const done = this.flight.settle(p, n);
      if (done) await this.land(p, done, values[p], false);
    }
  }

  async reset<P extends Partition>(partition: P): Promise<void> {
    this.v[partition] = await settingsIpc.reset(partition);
  }
}

export const settings = new SettingsStore();
