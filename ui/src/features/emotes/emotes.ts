// Small helpers for drawing provider emotes: names, hidden ones (E4), modifiers (C7).

import type { AddTarget } from '../../ipc/bindings/AddTarget';
import type { EmoteSource } from '../../ipc/bindings/EmoteSource';
import type { HereEdit } from '../../ipc/bindings/HereEdit';
import type { Layer } from '../../ipc/bindings/Layer';
import type { Provider } from '../../ipc/bindings/Provider';
import { MOVING, motion, slideSecs } from './phase.ts';

export const PROVIDER_NAMES: Record<Provider, string> = {
  twitch: 'Twitch',
  seventv: '7TV',
  bttv: 'BTTV',
  ffz: 'FFZ',
};

export function emoteKey(source: EmoteSource | Provider, id: string): string {
  return `${source}:${id}`;
}

/** C7 FFZ's modifier bits. */
export const MOD = {
  hidden: 1,
  flipX: 1 << 1,
  flipY: 1 << 2,
  growX: 1 << 3,
  slide: 1 << 4,
  appear: 1 << 5,
  leave: 1 << 6,
  rotate: 1 << 7,
  rotate90: 1 << 8,
  greyscale: 1 << 9,
  sepia: 1 << 10,
  rainbow: 1 << 11,
  hyperRed: 1 << 12,
  shake: 1 << 13,
  cursed: 1 << 14,
  jam: 1 << 15,
  bounce: 1 << 16,
  noSpace: 1 << 17,
  /** BTTV's own (crates/chat model::mods): `w!`, `l!`, `p!`, V111-20 `s!`. */
  wide: 1 << 18,
  rotateLeft: 1 << 19,
  party: 1 << 20,
  jitter: 1 << 21,
} as const;

/** V111-20 How an emote with these modifiers is drawn. */
export interface ModLook {
  /** `m-*` classes, the static look (emotes.css). */
  classes: string;
  /** Width over height when a modifier widens it (BTTV `w!` 4:1, FFZ GrowX twice its own, at most 4:1). */
  ratio: number | null;
  /** The CSS `animation` list of the moving ones, '' when none moves. */
  animation: string;
  /** FFZ Slide: drawn twice in a clipped box, the copies scroll left. */
  slide: boolean;
}

export function modLook(mods: number | undefined, size: readonly [number, number] | null | undefined): ModLook {
  const classes = modClasses(mods);
  if (!mods) return { classes, ratio: null, animation: '', slide: false };
  const own = size && size[1] > 0 ? size[0] / size[1] : 1;
  const ratio = mods & MOD.wide ? 4 : mods & MOD.growX ? Math.min(2 * own, 4) : null;
  const names = MOVING.filter((m) => mods & MOD[m]);
  return { classes, ratio, animation: motion(names, slideSecs(ratio ?? own)), slide: (mods & MOD.slide) !== 0 };
}

/** V102-06 Room for an emote before its picture loads: its size when the provider gave
 * one, else a square. `auto` lets the picture's own shape win once it's there, so a
 * 24×18 Twitch smiley like `:/` is never stretched to a square. */
export function emoteRatio(size: readonly [number, number] | null | undefined): string {
  return size ? `auto ${size[0]} / ${size[1]}` : 'auto 1 / 1';
}

/** Classes for an emote's modifiers; the look lives in `design/emotes.css`. */
export function modClasses(mods: number | undefined): string {
  if (!mods) return '';
  const out: string[] = [];
  for (const [name, bit] of Object.entries(MOD)) if (mods & bit && name !== 'hidden') out.push(`m-${name}`);
  return out.join(' ');
}

/** 1.41 What an emote card opens for. */
export interface CardTarget {
  emote: Layer;
  /** Zero-width emotes on top of it. */
  stack: Layer[];
  /** The emote's box on screen. */
  rect: DOMRect;
  /** 1.42 This is the emote that was gigantified in its message. */
  giant?: boolean;
  /** 1.41 Id of the message's author: their personal set may be where the emote came from. */
  sender?: string;
  /** V106-31/32 Opens with this form. */
  edit?: { kind: 'add'; target: AddTarget } | { kind: 'rename'; here: HereEdit };
}
