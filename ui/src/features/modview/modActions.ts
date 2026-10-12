// V103-15 The "Mod actions" panel's filters, shared by the mod view and the stream manager:
// kinds as chips (saved, like the activity feed's), a moderator and a user. Pure, tested.

import type { ModActionFilters } from '../../ipc/bindings/ModActionFilters';
import type { ModGroup } from '../../ipc/bindings/ModGroup';
import type { ModLine } from '../../ipc/bindings/ModLine';

/** Each kind's chip: its group (Rust's `ModGroup`), filter key and label. */
export const ACTION_KINDS: { group: ModGroup; key: keyof ModActionFilters; label: string }[] = [
  { group: 'ban', key: 'bans', label: 'Bans' },
  { group: 'timeout', key: 'timeouts', label: 'Timeouts' },
  { group: 'warn', key: 'warnings', label: 'Warnings' },
  { group: 'delete', key: 'deletions', label: 'Deletes' },
  { group: 'mode', key: 'modes', label: 'Modes' },
  { group: 'role', key: 'roles', label: 'Mods and VIPs' },
  { group: 'automod', key: 'automod', label: 'AutoMod' },
  { group: 'other', key: 'other', label: 'Other' },
];

const keyOf = new Map(ACTION_KINDS.map((k) => [k.group, k.key]));

export interface ActionQuery {
  filters: ModActionFilters;
  /** A moderator's login, or empty for any. */
  moderator: string;
  /** Part of the target's login or name, or empty for anyone. */
  user: string;
}

/** The lines the filters let through, newest first as Rust keeps them. */
export function visibleActions(lines: ModLine[], q: ActionQuery): ModLine[] {
  const user = q.user.trim().toLowerCase().replace(/^@/, '');
  return lines.filter((l) => {
    const key = keyOf.get(l.group);
    if (key && !q.filters[key]) return false;
    if (q.moderator && l.moderator?.login !== q.moderator) return false;
    if (user && !(l.target && (l.target.login.includes(user) || l.target.name.toLowerCase().includes(user)))) return false;
    return true;
  });
}

/** Everyone who did something in `lines`, A-Z, for the moderator filter. */
export function moderatorsIn(lines: ModLine[]): { login: string; name: string }[] {
  const seen = new Map<string, string>();
  for (const l of lines) if (l.moderator && !seen.has(l.moderator.login)) seen.set(l.moderator.login, l.moderator.name);
  return [...seen].map(([login, name]) => ({ login, name })).sort((a, b) => a.name.localeCompare(b.name));
}
