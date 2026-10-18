// V103-13 Settings in the Streaming group: Stream manager and Streamer mode.

import type { Entry } from './entry';

/** U4 What streamer mode hides. */
const hide = (field: string, label: string, description: string): Entry => ({
  id: `general.streamer.${field}`,
  page: 'streamer',
  label,
  description,
  keywords: ['streamer mode', 'hide'],
});

export const STREAMING: Entry[] = [
  // Stream manager
  {
    id: 'streammanager.obs.enabled',
    page: 'streammanager',
    label: 'Write status files',
    description: 'For OBS text sources. Updated about every 30 seconds.',
    keywords: ['obs', 'text source', 'viewers', 'uptime'],
  },
  {
    id: 'streammanager.obs.folder',
    page: 'streammanager',
    label: 'Status files folder',
    description: 'Pick where the OBS files go, or open the folder.',
    keywords: ['obs'],
  },
  {
    id: 'streammanager:files',
    page: 'streammanager',
    label: 'Files for OBS',
    description: 'Each file, what’s in it, and buttons to open it or copy its path.',
    keywords: ['obs', 'gdi', 'text source', 'path', 'markers'],
  },

  // Streamer mode
  {
    id: 'general.streamer.when',
    page: 'streamer',
    label: 'Streamer mode',
    description: 'Keeps things off your stream that viewers shouldn’t see.',
    keywords: ['streaming', 'obs', 'privacy'],
  },
  {
    id: 'general.streamer.apps',
    page: 'streamer',
    label: 'Streaming apps',
    description: 'Streamer mode turns on while one of these runs.',
    keywords: ['obs64.exe', 'streamlabs'],
    anchor: 'general.streamer.when',
  },
  hide('hideLinkPreviews', 'Hide link previews', 'Pictures and cards for links in chat.'),
  hide('hideAvatars', 'Hide profile pictures', 'On usercards.'),
  hide('hideModActions', 'Hide mod actions', 'Timeouts, bans and deleted messages.'),
  hide('hideWhispers', 'Hide whispers', 'In chat, and their pop-ups and sounds.'),
  hide('noNotifications', 'No Windows notifications', 'Live alerts, whispers and the bell.'),
  hide('muteSounds', 'Mute alerts', 'Mentions, highlights and whispers.'),
];
