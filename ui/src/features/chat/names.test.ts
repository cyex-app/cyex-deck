import assert from 'node:assert/strict';
import { test } from 'node:test';

import { shownName } from './names.ts';

const plain = { login: 'foo', display: 'Foo' };
const local = { login: 'sai', display: 'サイ' };

test('display name first, login in brackets only when it differs', () => {
  assert.deepEqual(shownName(plain, 'displayLogin'), { name: 'Foo' });
  assert.deepEqual(shownName(local, 'displayLogin'), { name: local.display, extra: 'sai' });
  assert.deepEqual(shownName(local, 'display'), { name: local.display });
});

test('username first, for everyone', () => {
  assert.deepEqual(shownName(plain, 'username'), { name: 'foo' });
  assert.deepEqual(shownName(local, 'username'), { name: 'sai' });
  assert.deepEqual(shownName(plain, 'usernameDisplay'), { name: 'foo' });
  assert.deepEqual(shownName(local, 'usernameDisplay'), { name: 'sai', extra: local.display });
});

test('a nickname wins', () => {
  assert.deepEqual(shownName(local, 'displayLogin', { nickname: 'Sai', note: '' }), { name: 'Sai' });
  assert.deepEqual(shownName(plain, 'displayLogin', { nickname: '', note: 'x' }), { name: 'Foo' });
});
