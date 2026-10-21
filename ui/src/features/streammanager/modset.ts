// V107-R38 The moderation settings panel's choices and edits, kept out of the component.

import type { ModsetPartial } from '../../ipc/bindings/ModsetPartial';
import type { ModsetVerification } from '../../ipc/bindings/ModsetVerification';
import type { ModsetVerifyMode } from '../../ipc/bindings/ModsetVerifyMode';
import { formatSecs } from '../../lib/format.ts';

/** AutoMod's categories in Rust's order (`CATEGORIES`), with Twitch's short names. */
export const CATEGORIES = [
  'Disability',
  'Aggression',
  'Sexuality, sex or gender',
  'Misogyny',
  'Bullying',
  'Swearing',
  'Race, ethnicity or religion',
  'Sexual harassment',
  'Sex-based terms',
  'Smart detection',
] as const;
/** Smart detection only takes these (Twitch's dashboard). */
const SMART = 9;

export const LEVELS = [
  { value: 0, label: 'Off' },
  { value: 1, label: '1' },
  { value: 2, label: '2' },
  { value: 3, label: '3' },
  { value: 4, label: '4' },
];

export function levelsFor(index: number): { value: string; label: string }[] {
  const all = LEVELS.map((l) => ({ value: String(l.value), label: l.value ? `Level ${l.value}` : 'Off' }));
  return index === SMART ? all.filter((l) => ['0', '2', '4'].includes(l.value)) : all;
}

export const DELAYS = [0, 2, 4, 6].map((s) => ({ value: s, label: s ? `${s}s` : 'Off' }));

export const MODES: { value: ModsetVerifyMode; label: string }[] = [
  { value: 'NONE', label: 'Nobody' },
  { value: 'ALL', label: 'Everyone' },
  { value: 'PARTIAL', label: 'Some' },
];

/** Minutes. */
export const ACCOUNT_AGES = [60, 1440, 10_080, 43_200, 129_600];
export const FOLLOW_AGES = [10, 60, 1440, 10_080, 43_200];
export const COOLDOWNS = [0, 15, 60, 360, 1440, 4320, 10_080];

/** Minute choices, with `current` added when it isn't one of them (set on Twitch). */
export function minuteChoices(list: number[], current: number | null): { value: string; label: string }[] {
  const all = current == null || list.includes(current) ? list : [...list, current].sort((a, b) => a - b);
  return all.map((m) => ({ value: String(m), label: m ? formatSecs(m * 60) : 'None' }));
}

export type Who = 'email' | 'phone';

export function withMode(v: ModsetVerification, who: Who, mode: ModsetVerifyMode): ModsetVerification {
  return { ...v, [who]: { ...v[who], mode } };
}

export function withPartial(v: ModsetVerification, who: Who, edit: Partial<ModsetPartial>): ModsetVerification {
  return { ...v, [who]: { ...v[who], partial: { ...v[who].partial, ...edit } } };
}

export type Exempt = 'exemptSubs' | 'exemptVips' | 'exemptMods';

export const EXEMPT: { key: Exempt; label: string }[] = [
  { key: 'exemptSubs', label: 'Subs' },
  { key: 'exemptVips', label: 'VIPs' },
  { key: 'exemptMods', label: 'Mods' },
];

/** Rules as Rust takes them (`clean_rules`): up to 15, each up to 150 characters. */
export const MAX_RULES = 15;
export const MAX_RULE = 150;

export function ruleProblem(text: string, rules: string[]): string | null {
  const t = text.trim();
  if (!t) return 'Type a rule.';
  if ([...t].length > MAX_RULE) return `A rule can have up to ${MAX_RULE} characters.`;
  if (rules.length >= MAX_RULES) return `Up to ${MAX_RULES} rules.`;
  if (rules.some((r) => r.toLowerCase() === t.toLowerCase())) return 'That rule is there already.';
  return null;
}

export const withRule = (rules: string[], text: string): string[] => [...rules, text.trim()];
export const withoutRule = (rules: string[], i: number): string[] => rules.filter((_, j) => j !== i);
