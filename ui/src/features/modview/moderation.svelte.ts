// Moderation in the UI (09): the role gate (1.57) and the mod menu entries shared by the
// chat name/message menus, mod mode's buttons and the usercard. Actions run as chat
// commands, so Helix or GQL is picked in Rust (D25).

import type { MenuEntry } from '../../components/menu.svelte';
import { formatDuration, formatSecs } from '../../lib/format';
import { settings } from '../../lib/settings.svelte';
import { toast } from '../../lib/toast.svelte';
import type { Caps } from '../../ipc/bindings/Caps';
import type { RoleEntry } from '../../ipc/bindings/RoleEntry';
import { modIpc, type ModRole } from '../../ipc/moderation';
import { pinCustomSecs } from './pinLength';
import { timeoutSecs } from './timeoutLength';
import { roleItems, type Standing } from './standing';
import { errorText } from '../../lib/errors';

/** 2.11 The quick timeout lengths from Settings → Chat, shortest first. */
export function timeouts(): { label: string; secs: number }[] {
  return settings.v.chat.timeoutPresets.map((secs) => ({ label: formatDuration(secs), secs }));
}

/** V101-34 How long a pin stays: Twitch takes 30 s to 30 min, or until the stream ends. */
export const PIN_LENGTHS: { label: string; secs: number | null }[] = [
  { label: 'Until the stream ends', secs: null },
  { label: formatSecs(30), secs: 30 },
  { label: formatSecs(60), secs: 60 },
  { label: formatSecs(300), secs: 300 },
  { label: formatSecs(600), secs: 600 },
  { label: formatSecs(1800), secs: 1800 },
];

class Mods {
  /** V102-21 Every open account's role per channel, as Rust keeps it; replaced on each
   * `mod:roles`, so mod controls come and go while a chat is open. */
  private list = $state.raw<RoleEntry[]>([]);
  private started = false;

  private start(): void {
    if (this.started) return;
    this.started = true;
    let events = 0;
    void modIpc
      .onRoles((list) => {
        events++;
        this.list = list;
      })
      .then(() => {
        const before = events;
        return modIpc.roles().then((list) => {
          if (events === before) this.list = list;
        });
      })
      .catch(() => {});
  }

  /** The account's role here, or null: anonymous, not a mod, or the chat isn't open. */
  role(account: string | null, channel: string): ModRole | null {
    this.start();
    if (!account) return null;
    const login = channel.toLowerCase();
    return this.list.find((e) => e.account === account && e.channel === login)?.role ?? null;
  }

  /** V107-R14 What the account may do here (Rust's answer, with the role), or null. */
  caps(account: string | null, channel: string): Caps | null {
    this.start();
    if (!account) return null;
    const login = channel.toLowerCase();
    return this.list.find((e) => e.account === account && e.channel === login)?.may ?? null;
  }

  /** 1.46 An open account that moderates the channel, the highest role first. */
  moderating(channel: string): string | null {
    this.start();
    const login = channel.toLowerCase();
    let best: RoleEntry | null = null;
    for (const e of this.list) {
      if (e.channel !== login) continue;
      if (!best || (e.may.roles && !best.may.roles) || e.may.broadcaster) best = e;
    }
    return best?.account ?? null;
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

/** Mod actions on a user, for the name menu (1.49) and the viewer list (C36). `warn` asks
 * for the reason: the chat box in a chat, the usercard elsewhere. */
export function userItems(caps: Caps, login: string, run: Run, warn: () => void, is?: Standing): MenuEntry[] {
  const items: MenuEntry[] = [
    {
      label: 'Time out',
      icon: 'clock',
      items: timeoutItems(login, run),
    },
    { label: 'Warn', icon: 'alert', run: warn },
    { label: 'Unban', icon: 'undo', run: () => run(`/unban ${login}`) },
    // V101-30 What can't be undone comes last.
    { label: 'Ban', icon: 'gavel', danger: true, run: () => run(`/ban ${login}`) },
    'separator',
    ...treatItems(caps, login, run, is),
  ];
  return items;
}

/** 2.11 The quick lengths, then V107-R22 any length Twitch takes, typed. */
export function timeoutItems(login: string, run: Run): MenuEntry[] {
  return [
    ...timeouts().map((t) => ({ label: t.label, run: () => run(`/timeout ${login} ${t.secs}`) })),
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

/** Monitor, restrict, and mod/VIP for the broadcaster and lead mods (1.50): the less used user actions. M12,
 * V107-M8 Mod or Unmod and VIP or Unvip from what they hold (`roleItems`). `roles: false` where the
 * role buttons show anyway (the usercard). */
export function treatItems(caps: Caps, login: string, run: Run, is?: Standing, roles = true): MenuEntry[] {
  const items: MenuEntry[] = [
    { label: 'Monitor', icon: 'eye', run: () => run(`/monitor ${login}`) },
    { label: 'Restrict', icon: 'eyeOff', run: () => run(`/restrict ${login}`) },
  ];
  if (roles && caps.roles) items.push('separator', ...roleItems(login, run, is));
  return items;
}

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

/** Mod actions on a message: pin, delete (1.49, 1.9). */
export function messageItems(msgId: string, run: Run): MenuEntry[] {
  return [
    { label: 'Pin message', icon: 'pin', items: pinItems(msgId, run) },
    { label: 'Delete message', icon: 'trash', danger: true, run: () => run(`/delete ${msgId}`) },
  ];
}

/** A convenience for components that need the toast-on-error runner. */
export function runner(send: (line: string) => Promise<unknown>): Run {
  return (line: string) => void send(line).catch((e) => toast.error(errorText(e)));
}
