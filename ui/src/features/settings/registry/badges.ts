// V108-N06 Settings of the extra badge sets, on the Emotes and badges page. Their own file so the
// Chat group's list stays about chat.

import type { Entry } from './entry';

const set = (key: string, name: string, description: string, keywords: string[] = []): Entry => ({
  id: `emotes.badgeSets.${key}`,
  page: 'emotes',
  label: `${name} badges`,
  description,
  keywords: ['badges', 'extra', ...keywords],
});

export const BADGES: Entry[] = [
  set('ffzap', 'FFZ:AP', 'Supporters of the FFZ add-on pack.', ['ffz ap', 'addon']),
  set('badgehq', 'BadgeHQ', 'Badges from badgehq.xyz.'),
  set('chatsen', 'Chatsen', 'People behind the Chatsen app and its supporters.'),
  set('dankchat', 'DankChat', 'People behind the DankChat app and its supporters.'),
  set('homies', 'Homies', 'Chatterino Homies, with the badges people made for themselves.', ['chatterino']),
];
