import assert from 'node:assert/strict';
import { test } from 'node:test';

import { markColor } from './markColor.ts';

test('no rule colour means the plain mark', () => {
  assert.equal(markColor([]), null);
  assert.equal(markColor([null, undefined, '']), null);
});

test('the latest valid colour wins', () => {
  assert.equal(markColor(['#ff8800', null]), '#ff8800');
  assert.equal(markColor(['#ff8800', '#00aaff']), '#00aaff');
});

test('only #rrggbb counts', () => {
  assert.equal(markColor(['red', '#fff', 'url(x)', '#ff880']), null);
  assert.equal(markColor(['#112233', 'red']), '#112233');
});
