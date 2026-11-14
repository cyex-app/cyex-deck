import assert from 'node:assert/strict';
import { test } from 'node:test';

import { splitEmote, splitNames } from './noticeNames.ts';

const u = (name: string, login: string) => ({ name, login });

test('gifter and recipient are cut out of the text', () => {
  assert.deepEqual(splitNames('Foo gifted a Tier 1 sub to Bar', [u('Foo', 'foo'), u('Bar', 'bar')]), [
    { text: 'Foo', login: 'foo' },
    { text: ' gifted a Tier 1 sub to ' },
    { text: 'Bar', login: 'bar' },
  ]);
});

test('matching ignores case and keeps the text as written', () => {
  assert.deepEqual(splitNames('FOO gifted', [u('foo', 'foo')]), [{ text: 'FOO', login: 'foo' }, { text: ' gifted' }]);
});

test('a name not in the text leaves it plain', () => {
  assert.deepEqual(splitNames('Someone gifted', [u('Foo', 'foo')]), [{ text: 'Someone gifted' }]);
  assert.deepEqual(splitNames('Foo gifted', []), [{ text: 'Foo gifted' }]);
});

test('a name inside a longer word does not match', () => {
  assert.deepEqual(splitNames('Bobby gifted to Bob', [u('Bob', 'bob')]), [
    { text: 'Bobby gifted to ' },
    { text: 'Bob', login: 'bob' },
  ]);
});

test('the same name twice takes the first spot, then the next', () => {
  const parts = splitNames('Ann gifted a sub to Ann', [u('Ann', 'ann'), u('Ann', 'ann')]);
  assert.equal(parts.filter((p) => p.login).length, 2);
});

test('CJK display names match', () => {
  assert.deepEqual(splitNames('テスト gifted a sub', [u('テスト', 'tesuto')]), [
    { text: 'テスト', login: 'tesuto' },
    { text: ' gifted a sub' },
  ]);
});

test('V111-01 the emote picture goes right after the emote name', () => {
  const at = (text: string, name: string) => splitEmote(text, name)?.join('|');
  assert.equal(at('xQc added the 7TV emote notL.', 'notL'), 'xQc added the 7TV emote notL|.');
  assert.equal(at('The 7TV emote notL was removed.', 'notL'), 'The 7TV emote notL| was removed.');
  // A rename shows the new name, also when the actor or the old name is the same word.
  assert.equal(at('xQc renamed the 7TV emote a to bee.', 'bee'), 'xQc renamed the 7TV emote a to bee|.');
  assert.equal(at('The BTTV emote a is now b.', 'b'), 'The BTTV emote a is now b|.');
  // BTTV names the maker: the maker is not the emote.
  assert.equal(at('The BTTV emote RareHug by RareHug was added.', 'RareHug'), 'The BTTV emote RareHug| by RareHug was added.');
  // A longer name that only starts the same doesn't count; a name not in the text gives nothing.
  assert.equal(at('The 7TV emote notLX was added.', 'notL'), 'The 7TV emote notL|X was added.');
  assert.equal(splitEmote('The 7TV emote x was added.', 'zzz'), null);
});

// V111-16 Mod action lines: the mod and the target both open their usercard, in their colours.
test('V111-16 a mod line links the mod and the target, with colours', () => {
  const color = { dark: '#ff0000', light: '#aa0000' };
  const parts = splitNames('Kuni and Bob timed out Ann for 5m (3 times).', [{ ...u('Kuni', 'kuni'), color }, u('Bob', 'bob'), u('Ann', 'ann')]);
  assert.deepEqual(
    parts.filter((p) => p.login).map((p) => [p.text, p.login, p.color?.dark]),
    [
      ['Kuni', 'kuni', '#ff0000'],
      ['Bob', 'bob', undefined],
      ['Ann', 'ann', undefined],
    ],
  );
});

test('V111-16 a name spelled as the line spells it wins over a word in another case', () => {
  const parts = splitNames('A deleted a message from A.', [u('A', 'a')]);
  assert.deepEqual(parts[0], { text: 'A', login: 'a' });
  assert.equal(parts.filter((p) => p.login).length, 1);
  // The target "a" in a line where the lower-case word comes first.
  const t = splitNames('Kuni deleted a message from A.', [u('A', 'a')]);
  assert.deepEqual(t.filter((p) => p.login).map((p) => p.text), ['A']);
  assert.equal(t.map((p) => p.text).join(''), 'Kuni deleted a message from A.');
});
