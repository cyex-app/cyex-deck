// A player's volume, kept per tile so a muted stream stays muted. Saved once a change
// settles; `hushed` silences it without saving, e.g. while a clip plays over it (1.20).
// V1051-01 Every player plays on the streams' device (output.svelte.ts).
// V105-10 Live and VOD players can go past 100 % with the boost (gain.ts).
// V106-25 A muted tab hushes it the same way (not saved); the player then shows muted, and
// unmuting it unmutes the tab: one mute, never two stacked.

import { untrack } from 'svelte';

import type { Tile } from '../../ipc/bindings/Tile';
import { sounding } from '../tabs/sounding.svelte';
import { tabs } from '../tabs/tabs.svelte';
import { clampVolume, split } from './boost';
import { releaseGain, setGain } from './gain';
import { applySink } from './output.svelte';

export interface Sound {
  readonly volume: number;
  /** Muted in the player or by its tab (V106-25). */
  readonly muted: boolean;
  readonly boost: boolean;
  /** From the volume control: anything but a mute unmutes the tab too (V106-25). */
  set: (volume: number, muted: boolean, commit: boolean) => void;
  /** V105-10 Turns the boost on or off; off brings the volume down to 100 % at most. */
  setBoost: (on: boolean) => void;
  /** P2 One scroll step, + louder. */
  step: (direction: number) => void;
  toggleMute: () => void;
  /** Puts the current sound on the element. Call in an effect. */
  apply: (video: HTMLMediaElement) => void;
  /** Drops the video's boost graph when the player goes. */
  release: (video: HTMLMediaElement) => void;
  /** V106-25 Reports the video to its tab's sound badge; returns the stop. */
  track: (video: HTMLMediaElement) => () => void;
}

/** `canBoost`: V105-10 live and VOD players; clips can't (their file plays silent through Web Audio). */
export function useSound(tile: () => Tile | undefined, hushed: () => boolean = () => false, canBoost = false): Sound {
  /** What the user is changing, until the tile has it. */
  let local = $state<{ volume: number; muted: boolean; boost: boolean } | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  const boost = $derived(canBoost && (local?.boost ?? tile()?.boost ?? false));
  const volume = $derived(clampVolume(local?.volume ?? tile()?.volume ?? 0.5, boost));
  const muted = $derived(local?.muted ?? tile()?.muted ?? false);
  /** V106-25 The tab holding the tile is muted. */
  const tabMuted = () => {
    const t = tile();
    return !!t && tabs.tileMuted(t.id);
  };
  function unmuteTab() {
    const t = tile();
    const tab = t && tabs.tabOf(t.id);
    if (tab && tabs.muted.has(tab.id)) void tabs.mute(tab.id, false);
  }
  /** What the user hears from the volume control: the player's own sound, or the tab's mute. */
  function change(v: number, m: boolean, commit: boolean) {
    if (!m) unmuteTab();
    set(v, m, commit);
  }

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
      return muted || tabMuted();
    },
    get boost() {
      return boost;
    },
    set: change,
    setBoost: (on) => set(on ? volume : Math.min(volume, 1), muted, true, on && canBoost),
    step: (direction) => change(Math.round((volume + direction * 0.05) * 100) / 100, false, true),
    toggleMute: () => change(volume || 0.5, !(muted || tabMuted()), true),
    apply: (video) => {
      const s = split(volume);
      video.volume = s.element;
      video.muted = muted || hushed() || tabMuted();
      applySink(video);
      if (canBoost) setGain(video, s.gain, boost);
    },
    /** The player's video is gone: drop its boost graph (V1051-01). */
    release: (video) => releaseGain(video),
    // Untracked: a tile edit (volume, boost) must not rerun the caller's effect, whose cleanup
    // drops the boost graph (V105-10).
    track: (video) =>
      untrack(() => {
        const t = tile();
        return t ? sounding.watch(t.id, video, () => !muted && volume > 0 && !hushed()) : () => {};
      }),
  };
}
