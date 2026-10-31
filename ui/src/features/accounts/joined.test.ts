import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ConnectionCount } from '../../ipc/bindings/ConnectionCount';
import { joinedGroups } from './joined.ts';

const name = (id: string) => (id === 'a' ? 'cyexbot' : 'caressz');
const count = (userId: string, channels: string[]): ConnectionCount => ({ userId, joined: channels.length, refused: [], channels });

test('the current account comes first, empty ones are left out', () => {
  const g = joinedGroups([count('a', ['x']), count('b', ['y']), count('c', [])], 'b', name, []);
  assert.deepEqual(g.map((x) => x.userId), ['b', 'a']);
});

test('channels with a stream first, then by login', () => {
  const g = joinedGroups([count('a', ['zed', 'amy', 'Bob'])], 'a', name, ['ZED']);
  assert.deepEqual(g[0].channels, [
    { login: 'zed', stream: true },
    { login: 'amy', stream: false },
    { login: 'Bob', stream: false },
  ]);
});

test('nothing joined gives no groups', () => {
  assert.deepEqual(joinedGroups([], 'a', name, ['x']), []);
});
