import assert from 'node:assert/strict';
import { test } from 'node:test';

import { shownName } from './names.ts';

const plain = { login: 'foo', display: 'Foo' };
const local = { login: 'sai', display: 'サイ' };

test('localized names get their login in "both" mode', () => {
  assert.deepEqual(shownName(plain, 'both'), { name: 'Foo' });
  assert.deepEqual(shownName(local, 'both'), { name: local.display, extra: 'sai' });
  assert.deepEqual(shownName(local, 'login'), { name: 'sai' });
  assert.deepEqual(shownName(local, 'display'), { name: local.display });
});

test('a nickname wins', () => {
  assert.deepEqual(shownName(local, 'both', { nickname: 'Sai', note: '' }), { name: 'Sai' });
  assert.deepEqual(shownName(plain, 'both', { nickname: '', note: 'x' }), { name: 'Foo' });
});
