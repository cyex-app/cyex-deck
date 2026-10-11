import assert from 'node:assert/strict';
import { test } from 'node:test';

import { parseLoginLine } from './manual.ts';

test("V101-08 a Chatterino login line fills every field", () => {
  assert.deepEqual(parseLoginLine('username=someone;user_id=42;client_id=abc;oauth_token=def;'), {
    login: 'someone',
    userId: '42',
    clientId: 'abc',
    token: 'def',
  });
});

test('V101-08 a lone token or name is not a login line', () => {
  assert.equal(parseLoginLine('oauth:abcdefghijklmnopqrstuvwxyz0123'), null);
  assert.equal(parseLoginLine('someone'), null);
  assert.equal(parseLoginLine('oauth_token=def'), null);
});
