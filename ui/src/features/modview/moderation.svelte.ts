// Moderation in the UI (09): the role gate (1.57) and the mod menu entries shared by the
// chat name/message menus and the usercard. Actions run as chat commands, so Helix or GQL
// is picked in Rust (D25).

import type { MenuEntry } from '../../components/menu.svelte';
import { toast } from '../../lib/toast.svelte';
import { modIpc, type ModRole } from '../../ipc/moderation';

/** Timeout presets for the mod menus and usercard (seconds, label). */
export const TIMEOUTS: { label: string; secs: number }[] = [
  { label: '1s', secs: 1 },
  { label: '30s', secs: 30 },
  { label: '1m', secs: 60 },
  { label: '5m', secs: 300 },
  { label: '10m', secs: 600 },
  { label: '1h', secs: 3600 },
  { label: '1d', secs: 86400 },
  { label: '1w', secs: 604800 },
];

class Mods {
  /** `${account}\n${channel}` → role, or null when checked and not a mod. */
  private roles = $state<Record<string, ModRole | null>>({});
  private pending = new Set<string>();

  private key(account: string | null, channel: string): string {
    return `${account ?? ''}\n${channel}`;
  }

  /** The account's role here; triggers a lookup the first time, re-reads live after. */
  role(account: string | null, channel: string): ModRole | null {
    if (!account) return null;
    const k = this.key(account, channel);
    if (!(k in this.roles) && !this.pending.has(k)) {
      this.pending.add(k);
      modIpc
        .role(account, channel)
        .then((r) => (this.roles[k] = r))
        .catch(() => (this.roles[k] = null))
        .finally(() => this.pending.delete(k));
    }
    return this.roles[k] ?? null;
  }

  /** Re-checks a role, e.g. after a reconnect changed it. */
  refresh(account: string | null, channel: string): void {
    if (!account) return;
    const k = this.key(account, channel);
    delete this.roles[k];
    this.role(account, channel);
  }
}

export const mods = new Mods();

/** Runs a mod line through the chat command path; toasts an error. */
type Run = (line: string) => void;

/** Mod actions on a user, for the name menu and usercard (1.49). */
export function userItems(role: ModRole, login: string, run: Run): MenuEntry[] {
  const items: MenuEntry[] = [
    {
      label: 'Timeout',
      icon: 'clock',
      items: TIMEOUTS.map((t) => ({ label: t.label, run: () => run(`/timeout ${login} ${t.secs}`) })),
    },
    { label: 'Ban', icon: 'ban', danger: true, run: () => run(`/ban ${login}`) },
    { label: 'Unban', icon: 'undo', run: () => run(`/unban ${login}`) },
    { label: 'Warn', icon: 'alert', run: () => run(`/warn ${login} `) },
    'separator',
    { label: 'Monitor', icon: 'eye', run: () => run(`/monitor ${login}`) },
    { label: 'Restrict', icon: 'eyeOff', run: () => run(`/restrict ${login}`) },
  ];
  if (role === 'broadcaster') {
    items.push('separator', { label: 'Mod', icon: 'shield', run: () => run(`/mod ${login}`) }, {
      label: 'VIP',
      icon: 'gem',
      run: () => run(`/vip ${login}`),
    });
  }
  return items;
}

/** Mod actions on a message: delete, pin (1.49, 1.9). */
export function messageItems(msgId: string, run: Run): MenuEntry[] {
  return [
    { label: 'Delete message', icon: 'trash', danger: true, run: () => run(`/delete ${msgId}`) },
    { label: 'Pin message', icon: 'pin', run: () => run(`/pin ${msgId}`) },
  ];
}

/** A convenience for components that need the toast-on-error runner. */
export function runner(send: (line: string) => Promise<unknown>): Run {
  return (line: string) => void send(line).catch((e) => toast.error(String(e)));
}
