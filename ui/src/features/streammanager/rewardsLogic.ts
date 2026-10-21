// V107-R37 The rewards panel's small rules: a new reward's defaults, the facts a row shows,
// and the queue's selection.

import type { SmReward } from '../../ipc/bindings/SmReward';
import type { SmRewardEdit } from '../../ipc/bindings/SmRewardEdit';
import { formatNumber, formatSecs } from '../../lib/format.ts';

/** Reward colors; the first is the app's default accent. */
export const COLORS = ['#145AFF', '#9147FF', '#00C7AC', '#1F69FF', '#FA2828', '#FF6905', '#FFCA5F', '#57BEE6', '#BFABFF', '#262626'];

export function blank(): SmRewardEdit {
  return {
    title: '',
    prompt: '',
    cost: 100,
    color: COLORS[0],
    enabled: true,
    inputRequired: false,
    skipQueue: false,
    cooldown: null,
    maxPerStream: null,
    maxPerUser: null,
  };
}

export function toEdit(r: SmReward): SmRewardEdit {
  return {
    title: r.title,
    prompt: r.prompt,
    cost: r.cost,
    color: r.color,
    enabled: r.enabled,
    inputRequired: r.inputRequired,
    skipQueue: r.skipQueue,
    cooldown: r.cooldown,
    maxPerStream: r.maxPerStream,
    maxPerUser: r.maxPerUser,
  };
}

export function sameEdit(a: SmRewardEdit, b: SmRewardEdit): boolean {
  return (Object.keys(a) as (keyof SmRewardEdit)[]).every((k) => a[k] === b[k]);
}

/** Short facts under a reward's name. */
export function facts(r: SmReward, now = Date.now()): string[] {
  const out: string[] = [];
  if (!r.enabled) out.push('Off');
  else if (r.paused) out.push('Paused');
  if (r.enabled && !r.inStock) out.push('Sold out');
  if (r.cooldownUntil && Date.parse(r.cooldownUntil) > now) out.push('Cooling down');
  else if (r.cooldown) out.push(`${formatSecs(r.cooldown)} cooldown`);
  if (r.maxPerStream) out.push(`${formatNumber(r.maxPerStream)} per stream`);
  if (r.maxPerUser) out.push(`${formatNumber(r.maxPerUser)} per viewer`);
  if (r.inputRequired) out.push('Asks for text');
  if (r.skipQueue) out.push('Skips the queue');
  if (r.subOnly) out.push('Subs only');
  return out;
}

/** "7 waiting", "50+ waiting". */
export function waiting(r: SmReward): string | null {
  if (!r.waiting) return null;
  return `${formatNumber(r.waiting)}${r.waitingMore ? '+' : ''} waiting`;
}

/** Turns one id on or off. */
export function toggle(sel: ReadonlySet<string>, id: string): Set<string> {
  const next = new Set(sel);
  if (!next.delete(id)) next.add(id);
  return next;
}

/** Keeps only ids still listed. */
export function keep(sel: ReadonlySet<string>, ids: readonly string[]): Set<string> {
  const listed = new Set(ids);
  return new Set([...sel].filter((id) => listed.has(id)));
}
