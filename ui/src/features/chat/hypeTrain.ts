// V108-N04 The hype train bar's pure parts: progress, time left, merging messages, the
// closing stats.

import type { HypeConductor } from '../../ipc/bindings/HypeConductor';
import type { HypeTrain } from '../../ipc/bindings/HypeTrain';

/** Progress to the next level, 0 to 100. */
export function progressPct(progress: number, goal: number): number {
  if (!(goal > 0) || !(progress > 0)) return 0;
  return Math.min(100, Math.round((progress / goal) * 100));
}

/** The level's points, "1.234 / 2.000"; "Goal reached" once the points pass the goal (the
 *  next level's message is on its way); null when Twitch sent no goal. */
export function progressText(progress: number, goal: number, fmt: (n: number) => string = String): string | null {
  if (!(goal > 0)) return null;
  if (progress >= goal) return 'Goal reached';
  return `${fmt(Math.max(0, progress || 0))} / ${fmt(goal)}`;
}

/** Whole seconds until `expiresAt` (RFC 3339), 0 once past or when unknown. */
export function secondsLeft(expiresAt: string | null, nowMs: number): number {
  if (!expiresAt) return 0;
  const t = Date.parse(expiresAt);
  return Number.isNaN(t) ? 0 : Math.max(0, Math.ceil((t - nowMs) / 1000));
}

/** "4:07". */
export function formatLeft(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

/** A progress message laid over the train we hold: it has no start time or conductors. */
export function mergeProgress(held: HypeTrain | null, next: HypeTrain, nowMs = Date.now()): HypeTrain {
  if (!held || (next.id && held.id && next.id !== held.id)) return next;
  return {
    ...held,
    level: next.level || held.level,
    progress: next.progress,
    goal: next.goal || held.goal,
    total: next.total || held.total,
    // A message without a time keeps the held one only while it is still ahead; a time
    // that has passed would end the train on the next tick.
    expiresAt: next.expiresAt ?? (secondsLeft(held.expiresAt, nowMs) > 0 ? held.expiresAt : null),
    goldenKappa: next.goldenKappa || held.goldenKappa,
    treasure: next.treasure || held.treasure,
  };
}

/** A changed conductor replaces the one of its source. */
export function mergeConductor(list: HypeConductor[], c: HypeConductor): HypeConductor[] {
  return [...list.filter((x) => x.source !== c.source), c];
}

/** The biggest giver as two parts, so a long name can be cut and the amount stays. */
export function topParts(list: HypeConductor[], fmt: (n: number) => string = String): { who: string; amount: string } | null {
  const top = [...list].filter((c) => c.name || c.login).sort((a, b) => b.total - a.total)[0];
  if (!top) return null;
  const unit = top.source === 'BITS' ? ' bits' : top.source === 'SUBS' ? ' subs' : '';
  return { who: top.name || top.login, amount: top.total > 0 ? `${fmt(top.total)}${unit}` : '' };
}

/** The biggest giver, "Name 1.200 bits". */
export function topLine(list: HypeConductor[], fmt: (n: number) => string = String): string | null {
  const t = topParts(list, fmt);
  return t ? (t.amount ? `${t.who} ${t.amount}` : t.who) : null;
}

/** The train as it closed: the last read laid over the one held, when it is the same train.
 * Subs and bits only come from reads, so the larger count wins. */
export function closing(held: HypeTrain, last: HypeTrain | null): HypeTrain {
  if (!last || (held.id && last.id && held.id !== last.id)) return held;
  return {
    ...held,
    level: last.level || held.level,
    progress: last.progress || held.progress,
    goal: last.goal || held.goal,
    total: Math.max(last.total, held.total),
    endedAt: last.endedAt ?? held.endedAt,
    conductors: last.conductors.length ? last.conductors : held.conductors,
    subs: Math.max(last.subs, held.subs),
    bits: Math.max(last.bits, held.bits),
  };
}

/** "24 subs · 5.300 bits"; null when neither is known. */
export function givenLine(subs: number, bits: number, fmt: (n: number) => string = String): string | null {
  const parts: string[] = [];
  if (subs > 0) parts.push(`${fmt(subs)} ${subs === 1 ? 'sub' : 'subs'}`);
  if (bits > 0) parts.push(`${fmt(bits)} ${bits === 1 ? 'bit' : 'bits'}`);
  return parts.length ? parts.join(' · ') : null;
}

/** "Level 3 reached" for the closing note; Twitch counts the level being worked on, so the
 * last one finished is one below. */
export function endedText(level: number): string {
  return level > 1 ? `Level ${level - 1} reached` : 'Hype train ended';
}

/** How long the topic may stay quiet before a read that finds no train ends the one we show. */
export const QUIET_MS = 90_000;

/** A read found no train: the one on screen is over when its time ran out or the topic has
 * been quiet for a while; otherwise the read is behind the topic. */
export function trainOver(expiresAt: string | null, nowMs: number, heardMs: number): boolean {
  return secondsLeft(expiresAt, nowMs) === 0 || nowMs - heardMs > QUIET_MS;
}
