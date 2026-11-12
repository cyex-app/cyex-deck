// V101-07 The settings list: five groups, each with its pages in order. SettingsView draws
// it; search (V103-13) shows a result's place as "group › page".

import type { IconName } from '../../../components/icons';

export const GROUPS = [
  {
    label: 'App',
    pages: [
      { id: 'general', label: 'General', icon: 'sliders', hint: 'Chat font, tray, smooth scrolling, Home and updates.' },
      { id: 'appearance', label: 'Appearance', icon: 'palette', hint: 'Theme, accent color, layout and tabs.' },
      { id: 'windows', label: 'Window layouts', icon: 'layout', hint: 'A layout remembers where the main window and every pop-out sit, and whether they’re pinned.' },
      { id: 'sound', label: 'Sound', icon: 'volume', hint: 'Which speakers play streams and alert sounds.' },
      { id: 'notifications', label: 'Alerts', icon: 'bell', hint: 'Pop-ups and sounds when channels go live or you get a whisper.' },
      { id: 'hotkeys', label: 'Hotkeys', icon: 'keyboard', hint: 'Click a shortcut, then press the new keys. Esc cancels, Backspace clears it.' },
      { id: 'menus', label: 'Menus', icon: 'list', hint: 'Right-click menus. Open one to rearrange it, rename or hide rows, and add groups or entries of your own.' },
      { id: 'plugins', label: 'Plugins', icon: 'puzzle', hint: 'Lua scripts that add commands, change chat lines and more. Each one only gets the permissions you agree to.' },
      { id: 'data', label: 'Backup and sharing', icon: 'archive', hint: 'Move your settings to another PC, share them, or go back to an earlier copy.' },
    ],
  },
  {
    label: 'Account',
    pages: [
      { id: 'accounts', label: 'Accounts', icon: 'user', hint: 'Who you chat and follow as. Each tab can use its own account.' },
    ],
  },
  {
    label: 'Chat',
    pages: [
      { id: 'chat', label: 'Chat', icon: 'chat', hint: 'How chat looks, loads and behaves.' },
      { id: 'emotes', label: 'Emotes and badges', icon: 'smile', hint: 'Which emotes and badges show, and from where.' },
      { id: 'highlights', label: 'Highlights', icon: 'star', hint: 'Make messages stand out, ping you, or copy them to the Mentions tab or a feed. For a feed, type its name.' },
      { id: 'ignores', label: 'Ignores', icon: 'eyeOff', hint: 'Hide messages and people you don’t want to see.' },
      { id: 'filters', label: 'Filters', icon: 'filter', hint: 'A chat shows only what passes its filters. Pick them from the chat’s menu.' },
      { id: 'commands', label: 'Commands', icon: 'pencil', hint: 'Your own chat commands and timers.' },
      { id: 'uploads', label: 'Image uploads', icon: 'image', hint: 'Images you paste or drop into the chat box go to your upload host. These settings stay on this PC and never go into exports.' },
      { id: 'automations', label: 'Automations', icon: 'zap', hint: 'When someone says the trigger, the response runs as if you typed it.' },
      { id: 'recordings', label: 'Recordings', icon: 'film', hint: 'Chat recordings you made: play or delete them.' },
    ],
  },
  {
    label: 'Watching',
    pages: [
      { id: 'player', label: 'Player', icon: 'tv', hint: 'Ads, latency, rewind and how much the player uses.' },
      { id: 'miner', label: 'Miner', icon: 'pickaxe', hint: 'Collects channel points while you watch and claims bonus chests.' },
    ],
  },
  {
    label: 'Streaming',
    pages: [
      { id: 'streammanager', label: 'Stream manager', icon: 'radio', hint: 'Text files for OBS with your live title and category.' },
      { id: 'streamer', label: 'Streamer mode', icon: 'monitor', hint: 'Keeps things off your stream that viewers shouldn’t see.' },
    ],
  },
  {
    label: 'About',
    pages: [{ id: 'about', label: 'About', icon: 'info', hint: 'Version, updates and connection health.' }],
  },
] as const satisfies readonly { label: string; pages: readonly { id: string; label: string; icon: IconName; hint: string }[] }[];

export type PageId = (typeof GROUPS)[number]['pages'][number]['id'];

export interface Page {
  id: PageId;
  label: string;
  icon: IconName;
  /** V110-N9 The line under the page title and the sidebar tooltip: one source for both. */
  hint: string;
  /** The group's name. */
  group: string;
}

/** Every page, in list order. */
export const PAGES: readonly Page[] = GROUPS.flatMap((g) => g.pages.map((p) => ({ ...p, group: g.label })));

export function pageOf(id: string): Page | undefined {
  return PAGES.find((p) => p.id === id);
}
