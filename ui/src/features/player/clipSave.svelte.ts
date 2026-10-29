// V108-N02 Clips being saved as MP4, by slug: progress for the buttons, and the one action
// that starts and ends a save (the dialog, the toast). The user starts every save.

import { playerIpc } from '../../ipc/player';
import { errorText } from '../../lib/errors';
import { toast } from '../../lib/toast.svelte';
import { type SaveProgress, saveText } from './clipSave';

class ClipSaves {
  /** Saves running now. A save waiting in the dialog has `done` 0. */
  active = $state<Record<string, SaveProgress>>({});

  has(slug: string): boolean {
    return slug in this.active;
  }

  text(slug: string): string {
    return saveText(this.active[slug]);
  }

  /** `height`: the file the player plays; null saves the best one. */
  async save(slug: string, height: number | null = null): Promise<void> {
    if (this.has(slug)) return;
    this.active[slug] = { done: 0, total: null };
    let off: (() => void) | undefined;
    try {
      off = await playerIpc.onClipSave((p) => {
        if (p.slug === slug && this.has(slug)) this.active[slug] = { done: p.done, total: p.total };
      });
      const path = await playerIpc.saveClip(slug, height);
      // null: cancelled in the dialog or while it saved.
      if (path) toast.success('Clip saved.');
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      off?.();
      delete this.active[slug];
    }
  }

  cancel(slug: string): void {
    void playerIpc.cancelClipSave(slug).catch(() => {});
  }
}

export const clipSaves = new ClipSaves();
