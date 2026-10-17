// Small helpers for drawing provider emotes: names, hidden ones (E4), modifiers (C7).

import type { AddTarget } from '../../ipc/bindings/AddTarget';
import type { EmoteSource } from '../../ipc/bindings/EmoteSource';
import type { HereEdit } from '../../ipc/bindings/HereEdit';
import type { Layer } from '../../ipc/bindings/Layer';
import type { Provider } from '../../ipc/bindings/Provider';

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
} as const;

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
  /** V106-31/32 Opens with this form. */
  edit?: { kind: 'add'; target: AddTarget } | { kind: 'rename'; here: HereEdit };
}
