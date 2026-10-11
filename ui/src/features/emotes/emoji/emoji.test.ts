import assert from 'node:assert/strict';
import { test } from 'node:test';

import { findEmoji, nameMap, parseGroups } from './index.ts';
import { emojiKey, splitEmoji } from './split.ts';

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

test('hover names have no colons and read as words (V102-07)', () => {
  const list = parseGroups([
    ['People & Body', '👍 +1 thumbsup|approve ok\n👀 eyes|look see watch'],
    ['Smileys & Emotion', '😍 heart_eyes|love crush\n❤️ heart|love'],
    ['Flags', '🇩🇪 de|flag germany'],
  ]);
  const map = nameMap(list);
  assert.equal(map.get(emojiKey('👀')), 'eyes');
  assert.equal(map.get(emojiKey('👍🏽')), 'thumbsup', 'skin tones share the name');
  assert.equal(map.get(emojiKey('😍')), 'heart eyes');
  assert.equal(map.get(emojiKey('❤')), 'heart', 'with or without the variation selector');
  assert.equal(map.get(emojiKey('🇩🇪')), 'flag germany');
});

test('emoji are found in chat text', () => {
  assert.deepEqual(splitEmoji('just text'), [{ text: 'just text', emoji: false }]);
  assert.deepEqual(
    splitEmoji('hi 👀👍🏽 and ❤️‍🔥!'),
    [
      { text: 'hi ', emoji: false },
      { text: '👀', emoji: true },
      { text: '👍🏽', emoji: true },
      { text: ' and ', emoji: false },
      { text: '❤️‍🔥', emoji: true },
      { text: '!', emoji: false },
    ],
  );
  assert.deepEqual(splitEmoji('🇩🇪1️⃣ ü'), [
    { text: '🇩🇪', emoji: true },
    { text: '1️⃣', emoji: true },
    { text: ' ü', emoji: false },
  ]);
});
