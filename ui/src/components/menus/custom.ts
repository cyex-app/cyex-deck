// V110-U67 Custom menu entries made without code: a link, a chat command, text for the
// message box, or text to copy. Values may hold placeholders (the same ones the S1 entries
// use); they are filled when the entry is picked.

import type { IconName } from '../icons';
import type { MenuEntry } from '../menuItems';
import type { CustomAction, CustomNode } from './types.ts';

export type Placeholder = 'nick' | 'display-nick' | 'user-id' | 'msg-id' | 'msg' | 'chan';
export type CustomKind = CustomAction['kind'];
export type Values = Partial<Record<Placeholder, string>>;

export const PLACEHOLDERS: { name: Placeholder; token: string; label: string }[] = [
  { name: 'nick', token: '$(nick)', label: 'Login name' },
  { name: 'display-nick', token: '$(display-nick)', label: 'Display name' },
  { name: 'user-id', token: '$(user-id)', label: 'User ID' },
  { name: 'msg-id', token: '$(msg-id)', label: 'Message ID' },
  { name: 'msg', token: '$(msg)', label: 'Message text' },
  { name: 'chan', token: '$(chan)', label: 'Channel' },
];

const NAMES = new Set<string>(PLACEHOLDERS.map((p) => p.name));
const USER: Placeholder[] = ['nick', 'display-nick', 'user-id'];
const MESSAGE: Placeholder[] = [...USER, 'msg-id', 'msg', 'chan'];

/** What each menu can fill in. The names in a chat's name menu come from a message, the viewer list's do not. */
const OFFERS: Record<string, Placeholder[]> = {
  'chat.name': [...USER, 'msg-id', 'msg', 'chan'],
  'chat.message': MESSAGE,
  'chat.emote': ['chan'],
  'chat.more': ['chan'],
  'feed.name': [...USER, 'chan'],
  'feed.message': MESSAGE,
  'tabs.tab': ['chan'],
  'tabs.tile': ['chan'],
  'tabs.bar': [],
  'tabs.group': [],
  player: ['chan'],
  'browse.card': ['chan'],
  'whispers.thread': USER,
  'clips.clip': ['chan'],
};

/** The placeholders a menu offers. `viewerList`: the chat's name menu opened from the viewer list, which has no message. */
export function placeholdersFor(menuId: string, opts: { viewerList?: boolean } = {}): Placeholder[] {
  const all = OFFERS[menuId] ?? [];
  return opts.viewerList ? all.filter((p) => p !== 'msg' && p !== 'msg-id') : all;
}

/** What a menu can do on pick. Chat commands and the message box need a chat. */
export function kindsFor(menuId: string): CustomKind[] {
  return menuId.startsWith('chat.') ? ['link', 'chat', 'prefill', 'copy'] : ['link', 'copy'];
}

export const KIND_LABELS: Record<CustomKind, string> = {
  link: 'Open link',
  chat: 'Run in chat',
  prefill: 'Put in message box',
  copy: 'Copy text',
};

export const KIND_ICONS: Record<CustomKind, IconName> = {
  link: 'link',
  chat: 'send',
  prefill: 'pencil',
  copy: 'copy',
};

const TOKEN = /\$\(([a-z-]+)\)/g;

/** Fills `$(name)` in a template. One pass, so a value that looks like a placeholder stays as typed. Unknown names are left alone, a missing value is empty. Links get URL-encoded values. */
export function fill(template: string, values: Values, mode: 'text' | 'url' = 'text'): string {
  return template.replace(TOKEN, (whole, name: string) => {
    if (!NAMES.has(name)) return whole;
    const v = values[name as Placeholder] ?? '';
    return mode === 'url' ? encodeURIComponent(v) : v;
  });
}

export const actionValue = (a: CustomAction): string => (a.kind === 'link' ? a.url : a.kind === 'chat' ? a.command : a.text);

export function makeAction(kind: CustomKind, value: string): CustomAction {
  switch (kind) {
    case 'link':
      return { kind, url: value.trim() };
    case 'chat':
      return { kind, command: value.trim() };
    default:
      return { kind, text: value };
  }
}

/** A short reason the entry can't be saved, or `null` when it can. */
export function validateCustom(menuId: string, label: string, action: CustomAction, opts: { viewerList?: boolean } = {}): string | null {
  if (!label.trim()) return 'Give it a name.';
  if (!kindsFor(menuId).includes(action.kind)) return 'This menu can’t do that.';
  const value = actionValue(action);
  if (!value.trim()) return action.kind === 'link' ? 'Add a link.' : 'Add the text.';
  const offered = placeholdersFor(menuId, opts);
  for (const m of value.matchAll(TOKEN)) {
    if (!NAMES.has(m[1])) return `$(${m[1]}) isn’t a placeholder.`;
    if (!offered.includes(m[1] as Placeholder)) return `$(${m[1]}) isn’t available in this menu.`;
  }
  if (action.kind === 'link') {
    const host = /^https?:\/\/([^/?#]*)/i.exec(value.trim());
    if (!host || !host[1]) return 'Links start with http:// or https://.';
    if (host[1].includes('$(')) return 'Placeholders go after the address, not in it.';
  }
  return null;
}

export interface Runners {
  openLink(url: string): void;
  /** Sends the text in the chat the menu belongs to. */
  chat(command: string): void;
  prefill(text: string): void;
  copy(text: string): void;
}

/** The menu entry for a custom node; picking it fills the placeholders and does the action. */
export function customEntry(node: CustomNode, values: Values, run: Runners): MenuEntry {
  const a = node.action;
  return {
    id: node.id,
    label: node.label,
    icon: (node.icon as IconName | null) ?? KIND_ICONS[a.kind],
    run: () => {
      if (a.kind === 'link') {
        const url = fill(a.url, values, 'url');
        // Only ever the web, whatever the template turned into.
        if (/^https?:\/\//i.test(url)) run.openLink(url);
      } else if (a.kind === 'chat') run.chat(fill(a.command, values));
      else if (a.kind === 'prefill') run.prefill(fill(a.text, values));
      else run.copy(fill(a.text, values));
    },
  };
}
