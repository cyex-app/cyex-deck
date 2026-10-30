import assert from 'node:assert/strict';
import { test } from 'node:test';

import { isDefaultLook, lookVars, withLook } from './look.ts';

test('a tab with no look follows Settings', () => {
  assert.equal(isDefaultLook(undefined), true);
  assert.equal(isDefaultLook({}), true);
  assert.equal(isDefaultLook({ chatSize: null, emoteSize: null }), true);
  assert.deepEqual(lookVars(undefined), {});
  assert.deepEqual(lookVars({ chatSize: null }), {});
});

test('only the set values become variables', () => {
  assert.deepEqual(lookVars({ chatSize: 18 }), { '--chat-size': '18px' });
  assert.deepEqual(lookVars({ chatSize: 14, emoteSize: 150 }), { '--chat-size': '14px', '--emote-scale': '1.5' });
  assert.equal(isDefaultLook({ emoteSize: 100 }), false);
});

test('changing one value keeps the other', () => {
  assert.deepEqual(withLook({ chatSize: 18, emoteSize: 150 }, { chatSize: null }), { chatSize: null, emoteSize: 150 });
  assert.deepEqual(withLook(undefined, { emoteSize: 200 }), { chatSize: null, emoteSize: 200 });
});
