// V108-N25 Moments on the player: where they sit on the seek bar, which ones the player can
// go back to, and where a clip of one ends. Pure; the chat finds them (chat:moments).

import type { Moment } from '../../ipc/bindings/Moment';
import type { MomentKind } from '../../ipc/bindings/MomentKind';
import { clock } from './time.ts';

/** A clip covers the seconds before its end: it ends this long after the moment was noticed,
 *  so the cause is in it (plan: moment uptime + 10 s). */
export const CLIP_LEAD = 10;
/** The most the player menu lists, newest first. */
export const MENU_MOMENTS = 30;
/** The most kept per player. */
export const KEEP = 200;
/** A label in the list is cut here. */
const LABEL_MAX = 36;

/** What the player can go back in: `on` when a rewind source exists, `min` its first second
 *  (since the stream started), `uptime` the stream's age now. */
export interface Source {
  on: boolean;
  min: number;
  uptime: number;
}

const NAMES: Record<MomentKind, string> = {
  spike: 'Chat spike',
  raid: 'Raid',
  gift: 'Gift bomb',
  announcement: 'Announcement',
  hype: 'Hype train',
  title: 'New title',
  category: 'New category',
  clips: 'Clip links',
};

/** "Chat spike · KEKW", "Raid"; long labels are cut. */
export function momentText(m: Pick<Moment, 'kind' | 'label'>): string {
  const label = m.label.trim().replace(/\s+/g, ' ');
  const name = NAMES[m.kind] ?? 'Moment';
  if (!label) return name;
  const cut = [...label].length > LABEL_MAX ? `${[...label].slice(0, LABEL_MAX - 1).join('')}…` : label;
  return `${name} · ${cut}`;
}

/** Seconds since the stream started: the one the chat filled in, else from the stream's start
 *  (ms). null when neither is known. */
export function secsOf(m: Pick<Moment, 'atMs' | 'streamSecs'>, startedAtMs: number | null): number | null {
  if (m.streamSecs !== null && Number.isFinite(m.streamSecs)) return Math.max(0, m.streamSecs);
  if (startedAtMs === null || Number.isNaN(startedAtMs)) return null;
  return Math.max(0, (m.atMs - startedAtMs) / 1000);
}

/** "Go there" only when a rewind source covers the second. */
export function canGoThere(secs: number | null, src: Source): boolean {
  return src.on && secs !== null && secs >= src.min && secs <= src.uptime;
}

/** Where a live clip of a moment at `secs` ends: the lead later, never past now. */
export function clipOffset(secs: number, uptime: number): number {
  return Math.max(0, Math.min(secs + CLIP_LEAD, uptime));
}

/** A live clip reaches back about this far (s); older moments need the archive video. */
export const LIVE_CLIP_REACH = 120;

export type MomentClip = { kind: 'live'; offset: number } | { kind: 'vod'; video: string; offset: number };

/** How to clip a moment at `secs`: the live broadcast while it is within reach, else the
 *  archive video (`video`, if the app knows it), else not at all. */
export function momentClip(secs: number, uptime: number, video: string | null): MomentClip | null {
  const offset = clipOffset(secs, uptime);
  if (uptime - offset <= LIVE_CLIP_REACH) return { kind: 'live', offset };
  return video ? { kind: 'vod', video, offset } : null;
}

/** New moments in, oldest first by id, no id twice, only the newest `KEEP`. */
export function mergeMoments(have: readonly Moment[], found: readonly Moment[]): Moment[] {
  const seen = new Set(have.map((m) => m.id));
  const added = found.filter((m) => !seen.has(m.id));
  if (!added.length) return have as Moment[];
  const all = [...have, ...added].sort((a, b) => a.id - b.id);
  return all.length > KEEP ? all.slice(all.length - KEEP) : all;
}

/** A mark on the seek bar: `t` seconds from the bar's start. */
export interface Mark {
  id: number;
  t: number;
  text: string;
  kind: MomentKind;
}

/** The marks for a bar that starts at `start` (stream seconds): only moments the bar can
 *  go to, since a mark you can't jump to is only noise. */
export function marks(list: readonly Moment[], startedAtMs: number | null, src: Source, start: number): Mark[] {
  if (!src.on) return [];
  const out: Mark[] = [];
  for (const m of list) {
    const secs = secsOf(m, startedAtMs);
    if (!canGoThere(secs, src) || secs === null || secs < start) continue;
    out.push({ id: m.id, t: secs - start, text: `${momentText(m)} · ${clock(secs)}`, kind: m.kind });
  }
  return out;
}

/** The newest `n`, newest first. */
export function newest(list: readonly Moment[], n = MENU_MOMENTS): Moment[] {
  return list.slice(-n).reverse();
}
