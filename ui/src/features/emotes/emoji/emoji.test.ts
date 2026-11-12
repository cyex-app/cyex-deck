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

test('the emoji itself finds it, with or without FE0F (U4-5)', () => {
  assert.deepEqual(findEmoji(groups, '😀').map((e) => e.char), ['😀']);
  const hearts = parseGroups([['Smileys & Emotion', '❤️ heart|love']]);
  assert.deepEqual(findEmoji(hearts, '❤').map((e) => e.char), ['❤️']);
  assert.deepEqual(findEmoji(hearts, '❤️').map((e) => e.char), ['❤️']);
});

test('hover names have no colons and keep underscores (V102-07)', () => {
  const list = parseGroups([
    ['People & Body', '👍 +1 thumbsup|approve ok\n👀 eyes|look see watch'],
    ['Smileys & Emotion', '😍 heart_eyes|love crush\n❤️ heart|love'],
    ['Flags', '🇩🇪 de|flag germany'],
  ]);
  const map = nameMap(list);
  assert.equal(map.get(emojiKey('👀')), 'eyes');
  assert.equal(map.get(emojiKey('👍🏽')), 'thumbsup', 'skin tones share the name');
  assert.equal(map.get(emojiKey('😍')), 'heart_eyes');
  assert.equal(map.get(emojiKey('❤')), 'heart', 'with or without the variation selector');
  assert.equal(map.get(emojiKey('🇩🇪')), 'flag_germany');
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

test('V110-F84 (c), (r) and TM stay text unless they carry the emoji selector', () => {
  for (const ch of ['©', '®', '™']) {
    assert.deepEqual(splitEmoji(`a ${ch} b`), [{ text: `a ${ch} b`, emoji: false }]);
    assert.deepEqual(splitEmoji(`a ${ch}️ b`), [
      { text: 'a ', emoji: false },
      { text: `${ch}️`, emoji: true },
      { text: ' b', emoji: false },
    ]);
  }
  assert.equal(splitEmoji('\u{1F440}')[0].emoji, true);
});

test('V110-N17 country flags answer to flag and flag_<country>', () => {
  const list = parseGroups([['Flags', '🏁 checkered_flag|race\n🇨🇭 switzerland\n🇩🇪 de|flag germany\n🏴󠁧󠁢󠁥󠁮󠁧󠁿 england']]);
  const found = findEmoji(list, 'flag').map((e) => e.char);
  assert.ok(found.includes('🇨🇭') && found.includes('🇩🇪') && found.includes('🏴󠁧󠁢󠁥󠁮󠁧󠁿'));
  assert.deepEqual(findEmoji(list, 'flag_sw').map((e) => e.char), ['🇨🇭']);
  assert.deepEqual(findEmoji(list, 'switz').map((e) => e.char), ['🇨🇭'], 'the country name still works');
  assert.equal(nameMap(list).get(emojiKey('🇨🇭')), 'switzerland');
  assert.equal(nameMap(list).get(emojiKey('🇩🇪')), 'flag_germany');
  assert.deepEqual(list[0].emoji[0].names, ['checkered_flag'], 'other flags stay as they are');
});
