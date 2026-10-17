// M9 The usercard's bell: a plain user highlight (Settings → Highlights) for this login.
import type { UserRule } from '../../ipc/bindings/UserRule';

const mine = (r: UserRule, login: string) => !r.regex && r.login.toLowerCase() === login.toLowerCase();

export function watched(users: UserRule[], login: string): boolean {
  return users.some((r) => r.enabled && mine(r, login));
}

/** Flips the user's highlight. An existing rule keeps its color and sound. */
export function toggleWatch(users: UserRule[], login: string): UserRule[] {
  const on = !watched(users, login);
  if (users.some((r) => mine(r, login))) {
    return users.map((r) => (mine(r, login) ? { ...r, enabled: on } : r));
  }
  return [...users, { enabled: true, login: login.toLowerCase(), regex: false, color: '', sound: false, soundFile: '', flash: false, show: false, notify: false, route: '' }];
}
