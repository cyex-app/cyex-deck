import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { EmoteMenu } from '../../ipc/bindings/EmoteMenu';
import type { MenuEmote } from '../../ipc/bindings/MenuEmote';
import { cellKey, pick, search, usable } from './menu.ts';

function e(provider: MenuEmote['provider'], id: string, name: string, locked: string | null = null): MenuEmote {
  return {
    provider,
    id,
    name,
    locked,
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
