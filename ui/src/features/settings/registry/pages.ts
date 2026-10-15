// V101-07 The settings list: five groups, each with its pages in order. SettingsView draws
// it; search (V103-13) shows a result's place as "group › page".

import type { IconName } from '../../../components/icons';

export const GROUPS = [
  {
    label: 'App',
    pages: [
      { id: 'general', label: 'General', icon: 'sliders' },
      { id: 'appearance', label: 'Appearance', icon: 'palette' },
      { id: 'windows', label: 'Windows', icon: 'layout' },
      { id: 'hotkeys', label: 'Hotkeys', icon: 'keyboard' },
      { id: 'plugins', label: 'Plugins', icon: 'puzzle' },
      { id: 'data', label: 'Backup and sharing', icon: 'archive' },
    ],
  },
  {
    label: 'Account',
    pages: [
      { id: 'accounts', label: 'Accounts', icon: 'user' },
      { id: 'notifications', label: 'Notifications', icon: 'bell' },
    ],
  },
  {
    label: 'Chat',
    pages: [
      { id: 'chat', label: 'Chat', icon: 'chat' },
      { id: 'emotes', label: 'Emotes and badges', icon: 'smile' },
      { id: 'highlights', label: 'Highlights', icon: 'star' },
      { id: 'mentions', label: 'Mentions', icon: 'at' },
      { id: 'ignores', label: 'Ignores', icon: 'eyeOff' },
      { id: 'filters', label: 'Filters', icon: 'filter' },
      { id: 'commands', label: 'Commands', icon: 'pencil' },
      { id: 'uploads', label: 'Image uploads', icon: 'image' },
      { id: 'automations', label: 'Automations', icon: 'zap' },
      { id: 'recordings', label: 'Recordings', icon: 'record' },
    ],
  },
  {
    label: 'Watching',
    pages: [
      { id: 'player', label: 'Player', icon: 'tv' },
      { id: 'miner', label: 'Miner', icon: 'pickaxe' },
    ],
  },
  {
    label: 'Streaming',
    pages: [
      { id: 'streammanager', label: 'Stream manager', icon: 'radio' },
      { id: 'streamer', label: 'Streamer mode', icon: 'monitor' },
    ],
  },
] as const satisfies readonly { label: string; pages: readonly { id: string; label: string; icon: IconName }[] }[];

export type PageId = (typeof GROUPS)[number]['pages'][number]['id'];

export interface Page {
  id: PageId;
  label: string;
  icon: IconName;
  /** The group's name. */
  group: string;
}

/** Every page, in list order. */
export const PAGES: readonly Page[] = GROUPS.flatMap((g) => g.pages.map((p) => ({ ...p, group: g.label })));

export function pageOf(id: string): Page | undefined {
  return PAGES.find((p) => p.id === id);
}
