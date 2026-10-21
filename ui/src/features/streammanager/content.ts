// V107-R39 What the Clips and Videos panels show, kept out of the components for tests.

import type { Clip } from '../../ipc/bindings/Clip';
import type { ManagedVideo } from '../../ipc/bindings/ManagedVideo';
import type { VideoKind } from '../../ipc/bindings/VideoKind';
import { formatDate, formatNumber } from '../../lib/format.ts';
import { clock } from '../player/time.ts';

/** What the panel player shows. */
export type Playing = { kind: 'clip'; slug: string } | { kind: 'vod'; id: string };

export const CLIP_PERIODS = [
  { value: 'day', label: '24h' },
  { value: 'week', label: '7d' },
  { value: 'month', label: '30d' },
  { value: 'all', label: 'All' },
] as const;

export const CLIP_ORDERS = [
  { value: 'views', label: 'Top' },
  { value: 'newest', label: 'New' },
] as const;

export const CLIPS_OF = [
  { value: 'channel', label: 'My channel' },
  { value: 'mine', label: 'Made by me' },
] as const;

export const VIDEO_KINDS = [
  { value: 'all', label: 'All' },
  { value: 'archive', label: 'Broadcasts' },
  { value: 'highlight', label: 'Highlights' },
  { value: 'upload', label: 'Uploads' },
] as const;

export type KindPick = (typeof VIDEO_KINDS)[number]['value'];

export function kindOf(pick: KindPick): VideoKind | null {
  return pick === 'all' ? null : pick;
}

const views = (n: number) => `${formatNumber(n)} ${n === 1 ? 'view' : 'views'}`;
const day = (iso: string) => {
  const t = Date.parse(iso);
  return Number.isNaN(t) ? '' : formatDate(new Date(t));
};

/** "by Name · 2026/10/20 · 1.200 views". */
export function clipLine(c: Clip, mine: boolean): string {
  const who = mine ? c.channelName && `in ${c.channelName}` : c.clippedBy && `by ${c.clippedBy}`;
  return [who, day(c.createdAt), views(c.views)].filter(Boolean).join(' · ');
}

export function clipLink(c: Clip): string {
  return c.channelLogin ? `https://www.twitch.tv/${c.channelLogin}/clip/${c.slug}` : `https://clips.twitch.tv/${c.slug}`;
}

export function videoLink(id: string): string {
  return `https://www.twitch.tv/videos/${id}`;
}

const KIND: Record<VideoKind, string> = { archive: 'Broadcast', highlight: 'Highlight', upload: 'Upload' };

/** "Broadcast · 2026/10/20 · 1.234 views". */
export function videoLine(v: ManagedVideo): string {
  return [KIND[v.kind], day(v.date), views(v.views)].filter(Boolean).join(' · ');
}

/** The tag after the title, if any: what isn't plain and published. */
export function videoTag(v: ManagedVideo): string | null {
  if (v.state === 'recording') return 'Live';
  if (v.state === 'processing') return 'Processing';
  if (v.state === 'failed') return 'Failed';
  return v.public ? null : 'Unpublished';
}

/** Length on the thumbnail: `1:02:03`. */
export function length(secs: number): string {
  return secs > 0 ? clock(secs) : '';
}

/** Flips one key in a selection. */
export function toggled(sel: ReadonlySet<string>, key: string): Set<string> {
  const next = new Set(sel);
  if (!next.delete(key)) next.add(key);
  return next;
}

/** The selection minus what's gone or no longer listed. */
export function kept(sel: ReadonlySet<string>, listed: readonly string[]): Set<string> {
  const here = new Set(listed);
  return new Set([...sel].filter((k) => here.has(k)));
}

/** One list after "Show more": new rows appended, repeats dropped. */
export function appended<T>(have: readonly T[], more: readonly T[], key: (t: T) => string): T[] {
  const seen = new Set(have.map(key));
  return [...have, ...more.filter((t) => !seen.has(key(t)))];
}

/** "Delete 3 clips" style counts. */
export function count(n: number, one: string, many: string): string {
  return `${formatNumber(n)} ${n === 1 ? one : many}`;
}
