// V110-U66 The mod rows of the name and message menus, one function each so a menu layout can
// place them one by one. Pure: the caller hands in what it knows (the account's caps, who the
// target is, the timeout lengths) and the function that runs a chat line.

import type { MenuEntry, MenuItem } from '../../components/menuItems.ts';
import { formatSecs } from '../../lib/format.ts';
import { pinCustomSecs } from './pinLength.ts';
import type { Standing } from './standing.ts';
import { roleItems } from './standing.ts';
import { timeoutSecs } from './timeoutLength.ts';

export type Run = (line: string) => void;

/** What the menu needs to offer mod actions on one user. */
export interface ModUser {
  login: string;
  /** Mod and VIP buttons: the broadcaster and lead mods. */
  roles: boolean;
  /** V107-R44 Time out and Ban on a mod: only the broadcaster, with a note. */
  onMod: { offered: boolean; note?: string };
  standing?: Standing;
  /** Quick timeout lengths, shortest first. */
  presets: { label: string; secs: number }[];
  run: Run;
  /** Asks for the reason of a warning. */
  warn: () => void;
}

/** 2.11 The quick lengths, then V107-R22 any length Twitch takes, typed. */
export function timeoutItems(login: string, run: Run, presets: { label: string; secs: number }[]): MenuEntry[] {
  return [
    ...presets.map((t) => ({ label: t.label, run: () => run(`/timeout ${login} ${t.secs}`) })),
    'separator',
    {
      label: 'Custom length',
      field: {
        placeholder: 'Custom, e.g. 7m or 1d',
        submit: (text: string) => {
          const t = timeoutSecs(text);
          if ('error' in t) return t.error;
          run(`/timeout ${login} ${t.secs}`);
          return null;
        },
      },
    },
  ];
}

export const timeoutEntry = (m: ModUser): MenuItem | null =>
  m.onMod.offered ? { label: 'Time out', icon: 'clock', note: m.onMod.note, items: timeoutItems(m.login, m.run, m.presets) } : null;

export const warnEntry = (m: ModUser): MenuItem => ({ label: 'Warn', icon: 'alert', run: m.warn });

export const unbanEntry = (m: ModUser): MenuItem => ({ label: 'Unban', icon: 'undo', run: () => m.run(`/unban ${m.login}`) });

export const banEntry = (m: ModUser): MenuItem | null =>
  m.onMod.offered ? { label: 'Ban', icon: 'gavel', danger: true, note: m.onMod.note, confirm: 'Click to ban', run: () => m.run(`/ban ${m.login}`) } : null;

export const monitorEntry = (m: ModUser): MenuItem => ({ label: 'Monitor', icon: 'eye', run: () => m.run(`/monitor ${m.login}`) });

export const restrictEntry = (m: ModUser): MenuItem => ({ label: 'Restrict', icon: 'eyeOff', run: () => m.run(`/restrict ${m.login}`) });

/** Mod or Unmod. */
export const modRoleEntry = (m: ModUser): MenuEntry | null => (m.roles ? (roleItems(m.login, m.run, m.standing)[0] as MenuItem) : null);

/** VIP or Unvip. */
export const vipRoleEntry = (m: ModUser): MenuEntry | null => (m.roles ? (roleItems(m.login, m.run, m.standing)[1] as MenuItem) : null);

/** V101-34 How long a pin stays: Twitch takes 30 s to 30 min, or no end. V108-C22 No end means
 * until a mod unpins it, live or offline, not until the stream ends. */
export const PIN_LENGTHS: { label: string; secs: number | null }[] = [
  { label: 'Until unpinned', secs: null },
  ...[30, 60, 300, 600, 1800].map((secs) => ({ label: formatSecs(secs), secs })),
];

/** V101-34 The pin lengths, for the message menu and the pin button; V107 then any length
 * Twitch takes, typed. */
export function pinItems(msgId: string, run: Run): MenuEntry[] {
  return [
    ...PIN_LENGTHS.map((p) => ({
      label: p.label,
      run: () => run(p.secs === null ? `/pin ${msgId}` : `/pin ${msgId} ${p.secs}s`),
    })),
    'separator',
    {
      label: 'Custom length',
      field: {
        placeholder: 'Custom, e.g. 12 or 2:30',
        submit: (text: string) => {
          const secs = pinCustomSecs(text);
          if (secs === null) return 'Try minutes, like 12 or 2:30.';
          run(`/pin ${msgId} ${secs}s`);
          return null;
        },
      },
    },
  ];
}

export const pinEntry = (msgId: string, run: Run): MenuItem => ({ label: 'Pin message', icon: 'pin', items: pinItems(msgId, run) });

export const deleteEntry = (msgId: string, run: Run): MenuItem => ({
  label: 'Delete message',
  icon: 'trash',
  danger: true,
  confirm: 'Click to delete',
  run: () => run(`/delete ${msgId}`),
});

/** Monitor, restrict, and mod/VIP (1.50): the less used user actions. `roles: false` where the
 * role buttons show anyway (the usercard). */
export function treatItems(roles: boolean, login: string, run: Run, standing?: Standing, showRoles = true): MenuEntry[] {
  const items: MenuEntry[] = [
    { label: 'Monitor', icon: 'eye', run: () => run(`/monitor ${login}`) },
    { label: 'Restrict', icon: 'eyeOff', run: () => run(`/restrict ${login}`) },
  ];
  if (showRoles && roles) items.push('separator', ...roleItems(login, run, standing));
  return items;
}
