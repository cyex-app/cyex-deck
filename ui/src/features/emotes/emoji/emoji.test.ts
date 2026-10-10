import assert from 'node:assert/strict';
import { test } from 'node:test';

import { findEmoji, parseGroups } from './index.ts';

const groups = parseGroups([
  ['Smileys & Emotion', '😀 grinning|smile happy\n😄 smile|happy joy\n😂 joy|tears'],
  ['Flags', '🏁 checkered_flag|race'],
]);

test('groups get short titles', () => {
  assert.equal(groups[0].title, 'Smileys');
  assert.deepEqual(groups[0].emoji[1], { char: '😄', names: ['smile'], words: 'happy joy' });
});

test('names first, then search words', () => {
  assert.deepEqual(
    findEmoji(groups, 'smi').map((e) => e.char),
    ['😄', '😀'],
  );
  assert.deepEqual(
    findEmoji(groups, 'joy').map((e) => e.char),
    ['😂', '😄'],
  );
});
