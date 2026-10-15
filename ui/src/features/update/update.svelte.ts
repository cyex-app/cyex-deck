// V105-20 The update badge's state: the release Rust found, and the progress of installing it.

import { Channel } from '@tauri-apps/api/core';

import type { Update } from '../../ipc/bindings/Update';
import type { UpdateProgress } from '../../ipc/bindings/UpdateProgress';
import { updateIpc } from '../../ipc/update';
import { progressText } from './updateText';

class UpdateStore {
  available = $state<Update | null>(null);
  progress = $state('');
  percent = $state<number | null>(null);
  error = $state('');
  busy = $state(false);
  private started = false;

  async start(): Promise<void> {
    if (this.started) return;
    this.started = true;
    await updateIpc.onAvailable((u) => {
      this.available = u;
      this.error = '';
    });
    this.available = await updateIpc.state().catch(() => null);
  }

  async install(): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    this.error = '';
    this.percent = 0;
    const on = new Channel<UpdateProgress>();
    on.onmessage = (p) => {
      this.progress = progressText(p);
      this.percent = p.kind === 'downloading' ? p.percent : 100;
    };
    try {
      await updateIpc.install(on);
    } catch (e) {
      this.error = String(e);
      this.progress = '';
      this.percent = null;
      this.busy = false;
    }
  }

  async skip(): Promise<void> {
    const v = this.available?.version;
    if (!v) return;
    await updateIpc.skip(v).catch((e) => (this.error = String(e)));
  }

  openNotes(): void {
    updateIpc.openNotes().catch((e) => (this.error = String(e)));
  }
}

export const update = new UpdateStore();
