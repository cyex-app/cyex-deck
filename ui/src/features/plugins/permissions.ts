// S5 What a plugin's permissions mean, in the words Settings shows before turning it on.

import type { IconName } from '../../components/icons';
import type { PluginMenuExport } from '../../ipc/bindings/PluginMenuExport';
import type { PluginPermission } from '../../ipc/bindings/PluginPermission';
import type { PluginStatus } from '../../ipc/bindings/PluginStatus';

export function permissionText(p: PluginPermission): { icon: IconName; text: string } {
  switch (p.type) {
    case 'ReadChat':
      return { icon: 'chat', text: 'Read chat in your open tabs' };
    case 'ChangeChat':
      return { icon: 'pencil', text: 'Read chat, and change or hide lines before you see them' };
    case 'SendMessages':
      return { icon: 'send', text: 'Send chat messages as you, and change or stop what you send' };
    case 'Network':
      return { icon: 'globe', text: p.hosts.length ? `Reach ${p.hosts.join(', ')}` : 'Reach any website' };
    case 'FilesystemRead':
      return { icon: 'folder', text: 'Read files in its own folder' };
    case 'FilesystemWrite':
      return { icon: 'download', text: 'Save files in its own folder' };
    case 'Tabs':
      return { icon: 'layout', text: 'Open channels in tabs and switch tabs' };
  }
}

const PLACE: Record<PluginMenuExport['place'], string> = { message: 'lines', chat: 'chat', name: 'names' };

/** The card's line about menu entries: "Weather › Forecast (chat), Who is this (names)". */
export function menuSummary(menu: PluginMenuExport[]): string {
  return menu.map((m) => `${m.group ? `${m.group} › ` : ''}${m.label} (${PLACE[m.place]})`).join(', ');
}

/** V107-R10 The switch shows what runs: one waiting for your OK on new permissions shows off. */
export function switchOn(saidOn: boolean, status: PluginStatus): boolean {
  return saidOn && status !== 'needsOk';
}

export const STATUS: Record<PluginStatus, { text: string; tone: 'ok' | 'muted' | 'warn' | 'bad' }> = {
  running: { text: 'Running', tone: 'ok' },
  off: { text: 'Off', tone: 'muted' },
  failed: { text: 'Didn’t start', tone: 'bad' },
  needsOk: { text: 'Needs your OK', tone: 'warn' },
  invalid: { text: 'Not a plugin', tone: 'bad' },
};
