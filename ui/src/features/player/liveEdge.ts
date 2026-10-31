// V1061-01..03 Live edge math: how far behind, where a rewind may land, what play does after a pause.

import type { HlsConfig } from 'hls.js';

import { clock } from './time.ts';

export const ENTER_BEHIND = 4; // s behind before the button says so
export const EXIT_BEHIND = 2; // s behind under which it's live again (no flicker)
export const PAUSE_STOP_MS = 10 * 60_000; // Q16 a
export const DVR_GAP = 20; // Twitch's `savvy-gap-seconds` default
export const DVR_YOUNG = 120; // s: ask Twitch again while the stream is this young
export const DVR_RETRY_MS = 15_000;
export const COALESCE_MS = 500; // Twitch's queueDVRSeek: presses add up, sent once

/** Seconds behind the live point, null when unknown. Never negative. */
export function behindLive(sync: number | null, current: number): number | null {
  if (sync === null || !Number.isFinite(sync) || !Number.isFinite(current)) return null;
  return Math.max(0, sync - current);
}

/** Hysteresis: 'live' until behind > ENTER_BEHIND, 'behind' until behind < EXIT_BEHIND. null keeps prev. */
export function liveState(prev: 'live' | 'behind', behind: number | null): 'live' | 'behind' {
  if (behind === null) return prev;
  if (prev === 'live') return behind > ENTER_BEHIND ? 'behind' : 'live';
  return behind < EXIT_BEHIND ? 'live' : 'behind';
}

/** Whole seconds since `startedAt` (RFC 3339), ≥ 0; null or a bad date → 0. */
export function uptimeAt(startedAt: string | null, now: number): number {
  if (!startedAt) return 0;
  const t = Date.parse(startedAt);
  if (Number.isNaN(t)) return 0;
  return Math.max(0, Math.floor((now - t) / 1000));
}

/** Bar seek: within DVR_GAP of uptime → 'live', else clamped to [min, uptime − DVR_GAP].
 *  `min`: where going back can start (0 for Twitch's rewind, the replay's start, V108-N24). */
export function dvrTarget(t: number, uptime: number, min = 0): number | 'live' {
  if (uptime - t < DVR_GAP) return 'live';
  return Math.max(min, t);
}

/** Key step from `base`. Back into the gap lands at uptime − DVR_GAP (A-L5); forward into it → 'live'. */
export function stepTarget(base: number, s: number, uptime: number, min = 0): { at: number | 'live'; edge: 'start' | null } {
  const t = base + s;
  if (s > 0 && uptime - t < DVR_GAP) return { at: 'live', edge: null };
  const max = uptime - DVR_GAP;
  if (max <= min) return { at: 'live', edge: 'start' };
  return { at: Math.min(max, Math.max(min, t)), edge: t < min ? 'start' : null };
}

/** V108-N24 Replay lengths offered in settings, minutes; 0 is off. */
export const REPLAY_MINUTES = [0, 2, 5, 10, 15, 30] as const;

/** V108-N24 The offered length closest to `n` (a hand-edited setting still shows). */
export function replayChoice(n: number): number {
  return REPLAY_MINUTES.reduce((best, m) => (Math.abs(m - n) < Math.abs(best - n) ? m : best), 5);
}

/** A max latency hls.js never reaches: it doesn't pull the player to the edge. */
export const NO_EDGE_PULL = 1e9;

/** d-1 hls.js's max latency. Paused, it never pulls the video to the edge: it seeked the
 *  paused video forward every 6-8 s, so the button read -0:06, -0:02... instead of counting up. */
export function maxLatency(lowLatency: boolean, paused: boolean): number {
  if (paused) return NO_EDGE_PULL;
  return lowLatency ? 12 : 20;
}

/** d-1 Paused at live: how far behind, counting up from the pause, one second a second. */
export function pausedBehind(atPause: number | null, pausedMs: number): number | null {
  return atPause === null ? null : atPause + Math.max(0, pausedMs) / 1000;
}

/** The live part of the hls.js config (today's values). */
export function liveConfig(lowLatency: boolean): Partial<HlsConfig> {
  const c: Partial<HlsConfig> = {
    // Segments still being made sit at the end: start two from the end.
    liveSyncDuration: lowLatency ? 4 : 8,
    liveMaxLatencyDuration: maxLatency(lowLatency, false),
    backBufferLength: 10,
    maxBufferLength: 12,
    maxMaxBufferLength: 20,
  };
  // V108-LL The segment still being made plays while it arrives (fetch streaming), as on
  // twitch.tv. Loaded whole, it played only once Twitch finished it: a full segment (2 s)
  // further behind. Same start point, so the buffer is never smaller than before.
  return lowLatency ? { ...c, progressive: true } : c;
}

/** The rewind VOD's config: an EVENT playlist hls.js would treat as live. A huge max
 *  latency keeps hls.js from pulling the player to the VOD's edge. */
export function dvrConfig(startAt: number): Partial<HlsConfig> {
  return {
    backBufferLength: 20,
    maxBufferLength: 30,
    maxMaxBufferLength: 40,
    startPosition: startAt,
    liveSyncDuration: 4,
    liveMaxLatencyDuration: NO_EDGE_PULL,
  };
}

/** Q15 a, Q16 a What play does after a pause. A pause at live comes back fully live, however
 *  short (user, 2026/10/31: no play-on under 1 s, no play-on from the N24 replay); only a
 *  deliberate rewind stays. */
export function resumePlan(o: { pausedMs: number; inVod: boolean; stopped: boolean }): 'sync' | 'reopen' | 'stay' | 'reload' {
  if (o.inVod) return o.stopped ? 'reload' : 'stay';
  if (o.stopped || o.pausedMs >= PAUSE_STOP_MS) return 'reopen';
  return 'sync';
}

/** s behind the live point a jump to live leaves as is (a seek that small only stutters). */
export const LIVE_SNAP = 0.3;

/** Whether going live has to move the play head to `sync` (2026/10/31: it used to leave up to 1 s). */
export function needsLiveSeek(sync: number, current: number): boolean {
  return sync - current >= LIVE_SNAP;
}

/** Q23 "Chat is live. The stream is 2:14 behind." */
export function chatBarText(behind: number): string {
  return `Chat is live. The stream is ${clock(behind)} behind.`;
}

/** V1061-01 U10 End of the buffered range that holds `t`, null when none does. */
export function rangeEnd(r: { length: number; start(i: number): number; end(i: number): number }, t: number): number | null {
  let end: number | null = null;
  for (let i = 0; i < r.length; i++) if (r.start(i) <= t && t <= r.end(i)) end = r.end(i);
  return end;
}
