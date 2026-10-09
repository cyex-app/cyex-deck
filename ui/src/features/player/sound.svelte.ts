// A player's volume, kept per tile so a muted stream stays muted. Saved once a change
// settles; `hushed` silences it without saving, e.g. while a clip plays over it (1.20).

import type { Tile } from '../../ipc/bindings/Tile';
import { tabs } from '../tabs/tabs.svelte';

export interface Sound {
  readonly volume: number;
  readonly muted: boolean;
  set: (volume: number, muted: boolean, commit: boolean) => void;
  /** P2 One scroll step, + louder. */
  step: (direction: number) => void;
  toggleMute: () => void;
  /** Puts the current sound on the element. Call in an effect. */
  apply: (video: HTMLMediaElement) => void;
}

export function useSound(tile: () => Tile | undefined, hushed: () => boolean = () => false): Sound {
  /** What the user is changing, until the tile has it. */
  let local = $state<{ volume: number; muted: boolean } | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  const volume = $derived(local?.volume ?? tile()?.volume ?? 0.5);
  const muted = $derived(local?.muted ?? tile()?.muted ?? false);

  function set(v: number, m: boolean, commit: boolean) {
    local = { volume: Math.min(1, Math.max(0, v)), muted: m };
    if (!commit) return;
    clearTimeout(timer);
    timer = setTimeout(() => {
      const t = tile();
      const next = local;
      if (!t || !next) return;
      void tabs.apply({ op: 'sound', tile: t.id, volume: next.volume, muted: next.muted }).then(() => {
        if (local === next) local = null;
      });
    }, 400);
  }

  return {
    get volume() {
      return volume;
    },
    get muted() {
      return muted;
    },
    set,
    step: (direction) => set(Math.round((volume + direction * 0.05) * 100) / 100, false, true),
    toggleMute: () => set(volume || 0.5, !muted, true),
    apply: (video) => {
      video.volume = volume;
      video.muted = muted || hushed();
    },
  };
}
