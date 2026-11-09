import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { EmoteMenu } from '../../ipc/bindings/EmoteMenu';
import type { MenuEmote } from '../../ipc/bindings/MenuEmote';
import { RANK } from '../chat/completion.ts';
import { search } from './menu.ts';
import { orderHits, rankEmotes, rankNames, usageRank } from './ranking.ts';

function e(provider: MenuEmote['provider'], id: string, name: string, locked: string | null = null): MenuEmote {
  return {
    provider,
    id,
    name,
    locked,
    tier: null,
    original: null,
    zeroWidth: false,
    modifier: null,
    unlisted: false,
    animated: false,
    size: null,
    owner: null,
    addedAt: null,
  };
}

const kapps: EmoteMenu = {
  twitch: [{ id: 'g', title: 'Global', avatar: null, emotes: [e('twitch', 'u', 'Kappu'), e('twitch', 'k', 'Kappa'), e('twitch', 'p', 'KappaPride')] }],
  twitchNote: null,
  seventv: [{ id: 's', title: 'Channel', avatar: null, emotes: [e('seventv', 'x', 'xKappx')] }],
  bttv: [],
  ffz: [],
};

test('V107-R7 menu search: exact, then prefix by use, then menu order; a used contains-match stays behind', () => {
  // Unused: menu order.
  assert.deepEqual(search(kapps, 'kapp').map((x) => x.name), ['Kappu', 'Kappa', 'KappaPride', 'xKappx']);
  const used = usageRank([
    { emote: { source: 'seventv', id: 'x', name: 'xKappx' } },
    { emote: { source: 'twitch', id: 'k', name: 'Kappa' } },
  ]);
  assert.deepEqual(search(kapps, 'kapp', 200, used).map((x) => x.name), ['Kappa', 'Kappu', 'KappaPride', 'xKappx']);
  // The exact name leads, even over one used more.
  const pride = usageRank([{ emote: { source: 'twitch', id: 'p', name: 'KappaPride' } }]);
  assert.deepEqual(search(kapps, 'kappa', 200, pride).map((x) => x.name), ['Kappa', 'KappaPride']);
});

test('V110-U71 Tab: the closest match first, the rest A-Z', () => {
  const names = ['MercyWing2', 'MercyWing1', 'merp', 'ME?', 'meow', 'menheraLost', 'melon', 'mewhenflang', 'xMe'];
  assert.deepEqual(rankNames('me', names, 30), [
    'ME?',
    'melon',
    'menheraLost',
    'meow',
    'MercyWing1',
    'MercyWing2',
    'merp',
    'mewhenflang',
    'xMe',
  ]);
  // The exact name in another case leads; what's typed exactly is left out; each name once.
  assert.deepEqual(rankNames('KEKW', ['KEKWait', 'KEKW', 'kekw', 'kekw', 'KEK'], 8), ['kekw', 'KEKWait']);
  // No exact name: the shortest prefix match leads (ties A-Z), the others follow A-Z.
  assert.deepEqual(rankNames('kek', ['KEKW', 'xKEKx', 'KEKL', 'KEKWait', 'kekB'], 8), ['kekB', 'KEKL', 'KEKW', 'KEKWait', 'xKEKx']);
  // Contains and acronym matches stay behind prefix ones.
  assert.deepEqual(rankNames('sob', ['SmallOrangeBird', 'xsobx', 'SoBayed']), ['SoBayed', 'xsobx', 'SmallOrangeBird']);
});

test('V110-U71 orderHits: the tie value decides between the same name', () => {
  const c = [
    { item: 'bttv', name: 'Same', tie: 2 },
    { item: 'seventv', name: 'Same', tie: 0 },
    { item: 'twitch', name: 'Same', tie: 1 },
    { item: 'other', name: 'Samey' },
  ];
  assert.deepEqual(orderHits('sam', c), ['seventv', 'twitch', 'bttv', 'other']);
  assert.deepEqual(orderHits('same', c), ['seventv', 'twitch', 'bttv', 'other']);
});

test('V107-R7 locked emotes come last, and the weakest match kept is the caller’s', () => {
  const c = [
    { item: 'a', name: 'xqcLocked', locked: true },
    { item: 'b', name: 'zxq' },
    { item: 'c', name: 'xaQ' },
  ];
  assert.deepEqual(rankEmotes('xq', c, RANK.contains), ['b', 'a']);
  assert.deepEqual(rankEmotes('xq', c), ['b', 'c', 'a']);
  assert.deepEqual(rankEmotes('  ', c), []);
  // The first place in the most-used list counts.
  assert.deepEqual([...usageRank([{ emote: { source: 'twitch', id: '1', name: 'a' } }, { emote: { source: 'twitch', id: '1', name: 'a' } }])], [['twitch:1', 0]]);
});

test('Tab skips names that only hold the letters in order (V110-F09)', () => {
  assert.deepEqual(rankNames('jul', ['jinxlul', 'juliet', 'xjulx']), ['juliet', 'xjulx']);
  assert.deepEqual(rankNames('jul', ['jinxlul']), []);
});
