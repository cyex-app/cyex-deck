// A player's volume, kept per tile so a muted stream stays muted. Saved once a change
// settles; `hushed` silences it without saving, e.g. while a clip plays over it (1.20).
// V105-10 Live and VOD players can go past 100 % with the boost (gain.ts).

import type { Tile } from '../../ipc/bindings/Tile';
import { tabs } from '../tabs/tabs.svelte';
import { clampVolume, split } from './boost';
import { setGain } from './gain';

export interface Sound {
  readonly volume: number;
  readonly muted: boolean;
  readonly boost: boolean;
  set: (volume: number, muted: boolean, commit: boolean) => void;
  /** V105-10 Turns the boost on or off; off brings the volume down to 100 % at most. */
  setBoost: (on: boolean) => void;
  /** P2 One scroll step, + louder. */
  step: (direction: number) => void;
  toggleMute: () => void;
  /** Puts the current sound on the element. Call in an effect. */
  apply: (video: HTMLMediaElement) => void;
}

/** `canBoost`: V105-10 live and VOD players; clips can't (their file plays silent through Web Audio). */
export function useSound(tile: () => Tile | undefined, hushed: () => boolean = () => false, canBoost = false): Sound {
  /** What the user is changing, until the tile has it. */
  let local = $state<{ volume: number; muted: boolean; boost: boolean } | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  const boost = $derived(canBoost && (local?.boost ?? tile()?.boost ?? false));
  const volume = $derived(clampVolume(local?.volume ?? tile()?.volume ?? 0.5, boost));
  const muted = $derived(local?.muted ?? tile()?.muted ?? false);

  function set(v: number, m: boolean, commit: boolean, b = boost) {
    local = { volume: clampVolume(v, b), muted: m, boost: b };
    if (!commit) return;
    clearTimeout(timer);
    timer = setTimeout(() => {
      const t = tile();
      const next = local;
      // V103-07 A player that isn't a tab's (the mod view's own) keeps its sound to itself.
      if (!t || !next || !tabs.tabOf(t.id)) return;
      // V105-10 A clip over a boosted stream (1.20) keeps the stream's boost.
      const boost = canBoost ? next.boost : t.boost;
      void tabs.apply({ op: 'sound', tile: t.id, volume: next.volume, muted: next.muted, boost }).then(() => {
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
    get boost() {
      return boost;
    },
    set,
    setBoost: (on) => set(on ? volume : Math.min(volume, 1), muted, true, on && canBoost),
    step: (direction) => set(Math.round((volume + direction * 0.05) * 100) / 100, false, true),
    toggleMute: () => set(volume || 0.5, !muted, true),
    apply: (video) => {
      const s = split(volume);
      video.volume = s.element;
      video.muted = muted || hushed();
      if (canBoost) setGain(video, s.gain, boost);
    },
  };
}
