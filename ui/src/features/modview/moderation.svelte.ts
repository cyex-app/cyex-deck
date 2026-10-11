// Moderation in the UI (09): the role gate (1.57) and the mod menu entries shared by the
// chat name/message menus, mod mode's buttons and the usercard. Actions run as chat
// commands, so Helix or GQL is picked in Rust (D25).

import type { MenuEntry } from '../../components/menu.svelte';
import { formatDuration } from '../../lib/format';
import { settings } from '../../lib/settings.svelte';
import { toast } from '../../lib/toast.svelte';
import { modIpc, type ModRole } from '../../ipc/moderation';

/** 2.11 The quick timeout lengths from Settings → Chat, shortest first. */
export function timeouts(): { label: string; secs: number }[] {
  return settings.v.chat.timeoutPresets.map((secs) => ({ label: formatDuration(secs), secs }));
}

/** V101-34 How long a pin stays: Twitch takes 30 s to 30 min, or until the stream ends. */
export const PIN_LENGTHS: { label: string; secs: number | null }[] = [
  { label: 'Until the stream ends', secs: null },
  { label: '30 seconds', secs: 30 },
  { label: '1 minute', secs: 60 },
  { label: '5 minutes', secs: 300 },
  { label: '10 minutes', secs: 600 },
  { label: '30 minutes', secs: 1800 },
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

  /** V101-33 Mod mode is on for this chat (remembered per channel). */
  modMode(channel: string): boolean {
    return settings.v.chat.modMode.includes(channel);
  }

  setModMode(channel: string, on: boolean): void {
    const list = settings.v.chat.modMode.filter((c) => c !== channel);
    void settings.patch('chat', { modMode: on ? [...list, channel] : list });
  }
}

export const mods = new Mods();

/** Runs a mod line through the chat command path; toasts an error. */
type Run = (line: string) => void;

/** Mod actions on a user, for the name menu (1.49). */
export function userItems(role: ModRole, login: string, run: Run): MenuEntry[] {
  const items: MenuEntry[] = [
    {
      label: 'Timeout',
      icon: 'clock',
      items: timeouts().map((t) => ({ label: t.label, run: () => run(`/timeout ${login} ${t.secs}`) })),
    },
    { label: 'Warn', icon: 'alert', run: () => run(`/warn ${login} `) },
    { label: 'Unban', icon: 'undo', run: () => run(`/unban ${login}`) },
    // V101-30 What can't be undone comes last.
    { label: 'Ban', icon: 'ban', danger: true, run: () => run(`/ban ${login}`) },
    'separator',
    ...treatItems(role, login, run),
  ];
  return items;
}

/** Monitor, restrict, and mod/VIP for the broadcaster: the less used user actions. */
export function treatItems(role: ModRole, login: string, run: Run): MenuEntry[] {
  const items: MenuEntry[] = [
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

/** V101-34 The pin lengths, for the message menu and the pin button. */
export function pinItems(msgId: string, run: Run): MenuEntry[] {
  return PIN_LENGTHS.map((p) => ({
    label: p.label,
    run: () => run(p.secs === null ? `/pin ${msgId}` : `/pin ${msgId} ${p.secs}s`),
  }));
}

/** Mod actions on a message: pin, delete (1.49, 1.9). */
export function messageItems(msgId: string, run: Run): MenuEntry[] {
  return [
    { label: 'Pin message', icon: 'pin', items: pinItems(msgId, run) },
    { label: 'Delete message', icon: 'trash', danger: true, run: () => run(`/delete ${msgId}`) },
  ];
}

/** A convenience for components that need the toast-on-error runner. */
export function runner(send: (line: string) => Promise<unknown>): Run {
  return (line: string) => void send(line).catch((e) => toast.error(String(e)));
}
