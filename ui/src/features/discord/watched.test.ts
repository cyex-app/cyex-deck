import assert from 'node:assert/strict';
import { test } from 'node:test';

import { watchedTile, watchOf } from './watched.ts';

const a = { id: 'a', channelId: '1' };
const b = { id: 'b', channelId: '2' };

test('nothing on air shows nothing', () => {
  assert.equal(watchedTile([a, b], [], null), null);
  assert.equal(watchedTile([], ['a'], null), null);
});

test('the tile with the sound wins when it is on air', () => {
  assert.equal(watchedTile([a, b], ['a', 'b'], 'b'), b);
});

test('a heard tile that is offline gives way to the first on air', () => {
  assert.equal(watchedTile([a, b], ['b'], 'a'), b);
});

test('no sound focus takes the first on air', () => {
  assert.equal(watchedTile([a, b], ['a', 'b'], null), a);
});

test('watchOf keeps https pictures only and falls back to the login', () => {
  assert.deepEqual(watchOf({ login: 'Bob', name: ' ', avatar: 'http://x/p.png' }), { login: 'bob', name: 'Bob', avatar: null });
  assert.equal(watchOf({ login: 'bob', name: 'Bob', avatar: 'https://x/p.png' }).avatar, 'https://x/p.png');
  assert.equal(watchOf({ login: 'bob', name: 'Bob', avatar: null }).avatar, null);
});
