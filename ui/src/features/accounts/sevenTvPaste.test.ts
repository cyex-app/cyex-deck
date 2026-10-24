import assert from 'node:assert/strict';
import { test } from 'node:test';

import { CONSOLE_LINE, looksLikeToken } from './sevenTvPaste.ts';

const FAKE = 'e30.eyJzdWIiOiIwMVgifQ.c2ln';

test('a pasted 7TV token is looked at before it is sent', () => {
  assert.equal(looksLikeToken(FAKE), true);
  assert.equal(looksLikeToken(`  ${FAKE}\n`), true);
  assert.equal(looksLikeToken(`"${FAKE}"`), true);
  assert.equal(looksLikeToken(`'${FAKE}'`), true);
  assert.equal(looksLikeToken(''), false);
  assert.equal(looksLikeToken('null'), false);
  assert.equal(looksLikeToken('a.b'), false);
  assert.equal(looksLikeToken('a..c'), false);
  assert.equal(looksLikeToken('a.b=.c'), false);
  assert.equal(looksLikeToken(`"${FAKE}`), false);
  assert.equal(looksLikeToken(`${'a'.repeat(600)}.${'b'.repeat(600)}.c`), false);
});

test('the console line is exactly the one 7TV keeps its session under', () => {
  assert.equal(CONSOLE_LINE, "copy(localStorage.getItem('7tv-token'))");
});
