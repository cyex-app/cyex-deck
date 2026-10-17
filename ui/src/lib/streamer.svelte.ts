// U4 Streamer mode as this window sees it. Rust decides when it's on (setting, streaming
// app running, a manual flip); the settings say what it hides.

import { modesIpc } from '../ipc/modes';
import type { StreamerBar } from '../ipc/bindings/StreamerBar';
import type { StreamerMode } from '../ipc/bindings/StreamerMode';
import type { StreamerState } from '../ipc/bindings/StreamerState';
import { settings } from './settings.svelte';

type Hides = keyof Pick<
  StreamerMode,
  'hideLinkPreviews' | 'hideAvatars' | 'hideModActions' | 'hideWhispers' | 'noNotifications' | 'muteSounds'
>;

class Streamer {
  state = $state<StreamerState>({ active: false, appRunning: false, forced: false, bar: null });

  get active(): boolean {
    return this.state.active;
  }

  /** V106-17 The bar to show in the main window, if any. */
  get bar(): StreamerBar | null {
    return this.state.active ? this.state.bar : null;
  }

  closeBar(): void {
    void modesIpc.closeStreamerBar().then((s) => (this.state = s));
  }

  /** Off until the streaming app stops (Rust clears the bar on a flip). */
  turnOff(): void {
    if (this.active) this.toggle();
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
