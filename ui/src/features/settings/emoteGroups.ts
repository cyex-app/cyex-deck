// V110-E13 The Emotes and badges page, one group per provider. Each child is a setting that
// already exists (its id and stored key stay); the group's switch is derived from them.

import { BADGE_SETS } from '../emotes/badgeSets.ts';

export interface Child {
  /** The registry id, also the row's `data-setting`. */
  id: string;
  label: string;
  hint?: string;
  part: 'chat' | 'emotes';
  /** Where it is stored inside that partition. */
  path: string[];
}

export interface Group {
  key: string;
  label: string;
  children: Child[];
}

type Values = { chat: unknown; emotes: unknown };

const chat = (id: string, label: string, path: string[], hint?: string): Child => ({
  id,
  label,
  hint,
  part: 'chat',
  path,
});

export const GROUPS: Group[] = [
  {
    key: 'twitch',
    label: 'Twitch',
    children: [chat('chat.twitchEmotes', 'Emotes', ['twitchEmotes']), chat('chat.badges.twitch', 'Badges', ['badges', 'twitch'])],
  },
  {
    key: 'seventv',
    label: '7TV',
    children: [
      chat('chat.seventvEmotes', 'Emotes', ['seventvEmotes'], 'Off: not loaded, drawn, suggested or ranked.'),
      chat('chat.badges.seventv', 'Badges', ['badges', 'seventv']),
      chat('chat.paints.seventv', 'Name paints', ['paints', 'seventv'], 'Colorful names for 7TV users.'),
    ],
  },
  {
    key: 'bttv',
    label: 'BTTV',
    children: [chat('chat.bttvEmotes', 'Emotes', ['bttvEmotes']), chat('chat.badges.bttv', 'Badges', ['badges', 'bttv'])],
  },
  {
    key: 'ffz',
    label: 'FFZ',
    children: [chat('chat.ffzEmotes', 'Emotes', ['ffzEmotes']), chat('chat.badges.ffz', 'Badges', ['badges', 'ffz'])],
  },
  {
    key: 'cyex',
    label: 'cyex',
    children: [chat('chat.badges.cyex', 'Badges', ['badges', 'cyex'], 'Founder, supporter and supporter+ badges from cyex deck.')],
  },
  {
    key: 'other',
    label: 'Other badges',
    children: BADGE_SETS.map((b) => ({
      id: `emotes.badgeSets.${b.key}`,
      label: b.label,
      hint: b.hint,
      part: 'emotes' as const,
      path: ['badgeSets', b.key],
    })),
  },
];

/** The 7TV paint outline: a detail of Name paints, not counted by the group. */
export const OUTLINE_ID = 'chat.paints.outlineLight';

export function read(v: Values, c: Child): boolean {
  let at: unknown = v[c.part];
  for (const k of c.path) at = (at as Record<string, unknown> | undefined)?.[k];
  return !!at;
}

/** The group a setting sits in, for the search to open it. */
export function groupOf(id: string): Group | undefined {
  if (id === OUTLINE_ID) return GROUPS.find((g) => g.key === 'seventv');
  return GROUPS.find((g) => g.children.some((c) => c.id === id));
}

/** The nested patch for `settings.patch`, per partition. */
export function patchFor(states: [Child, boolean][]): { chat?: object; emotes?: object } {
  const out: { chat?: Record<string, unknown>; emotes?: Record<string, unknown> } = {};
  for (const [c, on] of states) {
    let at = (out[c.part] ??= {});
    c.path.forEach((k, i) => {
      if (i === c.path.length - 1) at[k] = on;
      else at = (at[k] ??= {}) as Record<string, unknown>;
    });
  }
  return out;
}

/** On if any child is on. */
export const masterOn = (v: Values, g: Group): boolean => g.children.some((c) => read(v, c));

/** What the group switch changes: off turns every child off; on brings back the children that
 * were on before (`before`), or all of them when none are remembered. */
export function masterStates(g: Group, on: boolean, before?: Record<string, boolean>): [Child, boolean][] {
  if (!on) return g.children.map((c) => [c, false]);
  const kept = g.children.filter((c) => before?.[c.id]);
  const back = kept.length ? kept : g.children;
  return g.children.map((c) => [c, back.includes(c)]);
}

/** What is on right now, to bring back later. */
export const snapshot = (v: Values, g: Group): Record<string, boolean> =>
  Object.fromEntries(g.children.map((c) => [c.id, read(v, c)]));

/** "Emotes, badges · paints off" for a closed group. */
export function summary(v: Values, g: Group): string {
  const on = g.children.filter((c) => read(v, c)).map((c) => c.label.toLowerCase());
  const off = g.children.filter((c) => !read(v, c)).map((c) => c.label.toLowerCase());
  if (!on.length) return 'All off';
  const head = on.join(', ');
  const list = head.charAt(0).toUpperCase() + head.slice(1);
  return off.length ? `${list} · ${off.join(', ')} off` : list;
}

const KEY = 'cyex.emoteGroups';

function load(): Record<string, unknown> {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? '{}') as Record<string, unknown>;
  } catch {
    return {};
  }
}

function save(all: Record<string, unknown>): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(all));
  } catch {
    /* blocked storage: it just isn't remembered */
  }
}

/** Open or closed, remembered between runs. */
export const isOpen = (key: string): boolean => load()[`open:${key}`] === true;

export function setOpen(key: string, open: boolean): void {
  save({ ...load(), [`open:${key}`]: open });
}

export function memo(key: string): Record<string, boolean> | undefined {
  const m = load()[`prev:${key}`];
  return m && typeof m === 'object' ? (m as Record<string, boolean>) : undefined;
}

export function setMemo(key: string, states: Record<string, boolean>): void {
  save({ ...load(), [`prev:${key}`]: states });
}

export const OPEN_EVENT = 'cyex:open-emote-group';

/** A settings search picked `id`: its group opens (the page may not be mounted yet). */
export function openFor(id: string): void {
  const g = groupOf(id);
  if (!g) return;
  setOpen(g.key, true);
  window.dispatchEvent(new CustomEvent(OPEN_EVENT, { detail: g.key }));
}
