import assert from 'node:assert/strict';
import { test } from 'node:test';

import { checkAlias, judgeAlias } from './alias.ts';

test('good names pass', () => {
  for (const n of ['KEKW', 'peepo_Happy', ':)', '日本語', 'ok👍', 'a'.repeat(100)]) {
    assert.deepEqual(checkAlias(n), { ok: true, name: n }, n);
  }
});

test('bad names say why', () => {
  const msg = (t: string) => {
    const r = checkAlias(t);
    return r.ok ? null : r.message;
  };
  assert.equal(msg(''), 'Give it a name.');
  assert.equal(msg(' '), 'No spaces.');
  assert.equal(msg('a b'), 'No spaces.');
  assert.equal(msg('a'.repeat(101)), '100 characters at most.');
  for (const n of ['a/b', 'a@b', 'a,b']) assert.match(msg(n) ?? '', /^7TV doesn’t allow a character/, n);
});

test('judgeAlias: own rule first, then Rust, on the text given', async () => {
  const asked: string[] = [];
  const check = async (n: string) => (asked.push(n), n === 'Taken' ? 'Taken in x.' : null);
  assert.equal(await judgeAlias('a b', check), 'No spaces.');
  assert.deepEqual(asked, []);
  assert.equal(await judgeAlias('Taken', check), 'Taken in x.');
  assert.equal(await judgeAlias('Free', check), null);
  assert.deepEqual(asked, ['Taken', 'Free']);
  assert.equal(await judgeAlias('Free'), null);
});
