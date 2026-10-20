import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { EmoteMenu } from '../../ipc/bindings/EmoteMenu';
import type { MenuEmote } from '../../ipc/bindings/MenuEmote';
import { rankNames } from './ranking.ts';
import { completionEmotes } from './owned.ts';

function e(provider: MenuEmote['provider'], id: string, name: string, locked: string | null = null): MenuEmote {
  return { provider, id, name, locked, tier: null, original: null, zeroWidth: false, modifier: null, unlisted: false, animated: false, size: null, owner: null, addedAt: null };
}
const seen = (provider: MenuEmote['provider'], id: string, name: string) => ({ provider, id, name, animated: false });

const menu: EmoteMenu = {
  twitch: [
    {
      id: 'c',
      title: 'Chan',
      avatar: null,
      emotes: [e('twitch', '1', 'chanHi'), e('twitch', '2', 'chanTier2', 'Tier 2 sub'), e('twitch', '3', 'otherFollow', 'Sub to use')],
    },
  ],
  twitchNote: null,
  seventv: [{ id: 's', title: 'Channel', avatar: null, emotes: [e('seventv', 'a', 'chanWow')] }],
  bttv: [],
  ffz: [],
};

test('locked emotes never reach completion', () => {
  const names = completionEmotes(menu, []).map((x) => x.name);
  assert.deepEqual(names.sort(), ['chanHi', 'chanWow']);
  assert.deepEqual(rankNames('chan', names), ['chanHi', 'chanWow']);
  assert.deepEqual(rankNames('other', names), []);
});

test('a Twitch emote seen in chat counts only when owned', () => {
  const got = completionEmotes(menu, [seen('twitch', '9', 'strangerSub'), seen('twitch', '1', 'chanHi'), seen('seventv', 'z', 'chanSeen'), seen('twitch', '2', 'chanTier2')]);
  assert.deepEqual(got.map((x) => x.name).sort(), ['chanHi', 'chanHi', 'chanSeen', 'chanWow']);
});

test('signed out: no Twitch emotes, third party stays', () => {
  const out = { ...menu, twitch: [] };
  const got = completionEmotes(out, [seen('twitch', '9', 'strangerSub'), seen('bttv', 'b', 'okayChamp')]);
  assert.deepEqual(got.map((x) => x.name).sort(), ['chanWow', 'okayChamp']);
  assert.deepEqual(completionEmotes(null, [seen('twitch', '9', 'x')]), []);
});
