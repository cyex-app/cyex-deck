import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { EmoteMenu } from '../../ipc/bindings/EmoteMenu';
import type { MenuEmote } from '../../ipc/bindings/MenuEmote';
import { cellHint, cellKey, pick, search, searchCapped, searchGroups, tierMark, usable } from './menu.ts';

function e(
  provider: MenuEmote['provider'],
  id: string,
  name: string,
  locked: string | null = null,
  tier: number | null = null,
): MenuEmote {
  return {
    provider,
    id,
    name,
    locked,
    tier,
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

const menu: EmoteMenu = {
  twitch: [{ id: 't', title: 'Here', avatar: null, emotes: [e('twitch', '1', 'xqcL'), e('twitch', '2', 'xqcLocked', 'Tier 2')] }],
  twitchNote: null,
  seventv: [{ id: 's', title: 'Channel', avatar: null, emotes: [e('seventv', 'a', 'LUL'), e('seventv', 'b', 'xLULx')] }],
  bttv: [],
  ffz: [],
};

test('search ranks prefix, then contains, locked last', () => {
  assert.deepEqual(
    search(menu, 'xq').map((m) => m.name),
    ['xqcL', 'xqcLocked'],
  );
  assert.deepEqual(
    search(menu, 'lul').map((m) => m.name),
    ['LUL', 'xLULx'],
  );
});

test('favorites only show when usable here', () => {
  const have = usable(menu);
  const favs = pick(
    [
      { source: 'seventv', id: 'b', name: 'xLULx' },
      { source: 'twitch', id: '2', name: 'xqcLocked' },
      { source: 'ffz', id: '9', name: 'gone' },
    ],
    have,
  );
  assert.deepEqual(
    favs.map((m) => m.name),
    ['xLULx'],
  );
});

const aliased: EmoteMenu = {
  twitch: [],
  twitchNote: null,
  seventv: [
    { id: 's', title: 'Channel', avatar: null, emotes: [e('seventv', 'A', 'uncPls'), e('seventv', 'A', 'Oldheaddance')] },
  ],
  bttv: [],
  ffz: [],
};

test('V105-04 cellKey differs for one emote under two names', () => {
  assert.notEqual(cellKey(e('seventv', 'A', 'uncPls')), cellKey(e('seventv', 'A', 'Oldheaddance')));
});

test('V105-04 both aliases can be found', () => {
  assert.deepEqual(search(aliased, 'pls').map((m) => m.name), ['uncPls']);
  assert.deepEqual(search(aliased, 'old').map((m) => m.name), ['Oldheaddance']);
});

test('V105-04 usable keeps one entry per emote id', () => {
  const u = usable(aliased);
  assert.equal(u.size, 1);
  assert.ok(u.has('seventv:A'));
});

test('1.40 tier marks: T2 and T3 on Twitch sub emotes, none for tier 1', () => {
  assert.equal(tierMark(e('twitch', '1', 'a', null, 1)), null);
  assert.equal(tierMark(e('twitch', '2', 'b', null, 2)), 'T2');
  assert.equal(tierMark(e('twitch', '3', 'c', 'Tier 3 sub', 3)), 'T3');
  assert.equal(tierMark(e('seventv', '4', 'd', null, 2)), null);
  assert.equal(tierMark(e('twitch', '5', 'e')), null);
});

test('E5 hints: locked says why, then the tier, then the provider', () => {
  assert.equal(cellHint(e('twitch', '3', 'c', 'Tier 3 sub', 3)), 'Tier 3 sub');
  assert.equal(cellHint(e('twitch', '2', 'b', null, 2)), 'Tier 2 emote');
  assert.equal(cellHint(e('twitch', '1', 'a', null, 1)), 'Twitch');
  assert.equal(cellHint(e('bttv', 'x', 'y')), 'BTTV');
});

test('E5 locked emotes are never usable, so favorites and most used skip them', () => {
  const m: EmoteMenu = {
    twitch: [{ id: 't', title: 'Here', avatar: null, emotes: [e('twitch', '9', 'own', null, 2), e('twitch', '8', 'shut', 'Tier 3 sub', 3)] }],
    twitchNote: null,
    seventv: [],
    bttv: [],
    ffz: [],
  };
  const have = usable(m);
  assert.deepEqual([...have.keys()], ['twitch:9']);
  assert.deepEqual(pick([{ source: 'twitch', id: '8', name: 'shut' }], have), []);
});

test('V107-My search groups by platform, ranked inside each, empty ones dropped', () => {
  const g = searchGroups(menu, 'l');
  assert.deepEqual(g.map((x) => x.tab), ['twitch', 'seventv']);
  assert.deepEqual(g[1].emotes.map((m) => m.name), ['LUL', 'xLULx']);
  assert.equal(searchGroups(menu, 'l', 2).reduce((n, x) => n + x.emotes.length, 0), 2);
});

test('V107-H7 the "Showing 200" hint only when matches were cut', () => {
  const total = searchGroups(menu, 'l', 100).reduce((n, x) => n + x.emotes.length, 0);
  assert.ok(total >= 3);
  assert.equal(searchCapped(menu, 'l', total).cut, false);
  assert.equal(searchCapped(menu, 'l', total + 5).cut, false);
  const c = searchCapped(menu, 'l', total - 1);
  assert.equal(c.cut, true);
  assert.equal(c.groups.reduce((n, x) => n + x.emotes.length, 0), total - 1);
});
