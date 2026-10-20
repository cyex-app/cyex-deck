import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { EmoteMenu } from '../../ipc/bindings/EmoteMenu';
import type { MenuEmote } from '../../ipc/bindings/MenuEmote';
import { RANK } from '../chat/completion.ts';
import { search } from './menu.ts';
import { rankEmotes, rankNames, usageRank } from './ranking.ts';

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

test('V107-R7 Tab completion ranks the same way', () => {
  const used = (n: string) => ({ KEKWait: 0, KEKL: 1, xKEKx: 0 })[n] ?? Infinity;
  const names = ['KEKW', 'xKEKx', 'KEKL', 'KEKWait'];
  assert.deepEqual(rankNames('kek', names, 8, used), ['KEKWait', 'KEKL', 'KEKW', 'xKEKx']);
  // Unused: the order given (menu order), not the shortest.
  assert.deepEqual(rankNames('kek', names), ['KEKW', 'KEKL', 'KEKWait', 'xKEKx']);
  // Exact case leads; what's typed exactly is left out; each name once.
  assert.deepEqual(rankNames('KEKW', ['KEKWait', 'KEKW', 'kekw', 'kekw'], 8, used), ['kekw', 'KEKWait']);
});

test('V107-R7 one order for both: menu and Tab agree', () => {
  const used = usageRank([
    { emote: { source: 'seventv', id: 'x', name: 'xKappx' } },
    { emote: { source: 'twitch', id: 'p', name: 'KappaPride' } },
  ]);
  const byName = new Map([['xKappx', 0], ['KappaPride', 1]]);
  const tab = rankNames('kapp', ['Kappu', 'Kappa', 'KappaPride', 'xKappx'], 8, (n) => byName.get(n) ?? Infinity);
  assert.deepEqual(search(kapps, 'kapp', 200, used).map((x) => x.name), tab);
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
