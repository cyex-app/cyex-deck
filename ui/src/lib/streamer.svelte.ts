// U4 Streamer mode as this window sees it. Rust decides when it's on (setting, streaming
// app running, a manual flip); the settings say what it hides.

import { modesIpc } from '../ipc/modes';
import type { StreamerMode } from '../ipc/bindings/StreamerMode';
import type { StreamerState } from '../ipc/bindings/StreamerState';
import { settings } from './settings.svelte';

type Hides = keyof Pick<
  StreamerMode,
  'hideLinkPreviews' | 'hideAvatars' | 'hideModActions' | 'hideWhispers' | 'noNotifications' | 'muteSounds'
>;

class Streamer {
  state = $state<StreamerState>({ active: false, appRunning: false, forced: false });

  get active(): boolean {
    return this.state.active;
  }

  /** Whether streamer mode is on and hides this. */
  hides(what: Hides): boolean {
    return this.state.active && !!settings.v.general?.streamer?.[what];
  }

  toggle(): void {
    void modesIpc.toggleStreamer().then((s) => (this.state = s));
  }

  start(): () => void {
    let stop: (() => void) | undefined;
    let closed = false;
    void modesIpc.streamer().then((s) => (this.state = s));
    void modesIpc
      .onStreamer((s) => (this.state = s))
      .then((un) => (closed ? un() : (stop = un)));
    return () => {
      closed = true;
      stop?.();
    };
  }
}

export const streamer = new Streamer();
