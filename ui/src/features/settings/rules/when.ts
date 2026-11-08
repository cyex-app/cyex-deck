// V110-C59 A rule's conditions: the short summary on its chip, and edits that drop back to
// "always" (no `when`) once nothing is set.

import type { When } from '../../../ipc/bindings/When';

export const NO_WHEN: When = {
  channels: [],
  notChannels: false,
  categories: [],
  live: null,
  role: null,
  burst: 0,
  burstSecs: 0,
  firstOnly: false,
};

export function isAlways(w: When | null | undefined): boolean {
  return (
    !w ||
    (!w.channels.length && !w.categories.length && !w.live && !w.role && w.burst < 2 && !w.firstOnly)
  );
}

/** The `when` to store: `null` when nothing holds anything. */
export function cleanWhen(w: When): When | null {
  return isAlways(w) ? null : w;
}

const ROLE = { vip: 'as VIP', mod: 'as mod', broadcaster: 'in your chat' } as const;

/** "Only in 3 channels · live", or `null` when the rule always applies. */
export function whenSummary(w: When | null | undefined): string | null {
  if (!w || isAlways(w)) return null;
  const parts: string[] = [];
  const n = w.channels.length;
  if (n) {
    const which = n === 1 ? w.channels[0] : `${n} channels`;
    parts.push(w.notChannels ? `Not in ${which}` : `Only in ${which}`);
  }
  const c = w.categories.length;
  if (c) parts.push(c === 1 ? w.categories[0] : `${c} categories`);
  if (w.live) parts.push(w.live);
  if (w.role) parts.push(ROLE[w.role]);
  if (w.burst >= 2) parts.push(`${w.burst} in ${w.burstSecs || 1} s`);
  if (w.firstOnly) parts.push('first message');
  const out = parts.join(' · ');
  return out.charAt(0).toUpperCase() + out.slice(1);
}

/** Channels: "a, #b c" → ["a", "b", "c"]. */
export function splitChannels(text: string): string[] {
  return [...new Set(text.split(/[\s,]+/).map((s) => s.replace(/^[#@]/, '').trim().toLowerCase()).filter(Boolean))];
}

/** Categories have spaces ("Just Chatting"): commas only. */
export function splitCategories(text: string): string[] {
  return [...new Set(text.split(',').map((s) => s.trim()).filter(Boolean))];
}
