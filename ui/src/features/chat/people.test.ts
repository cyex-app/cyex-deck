import assert from 'node:assert/strict';
import { test } from 'node:test';

import { matchPeople, mentionName, mentionRank, tabNames, type Person } from './people.ts';

const people: Person[] = [
  { name: 'Alice', display: 'Alice' },
  { name: 'rtl_arabic', display: 'مستخدم_تجريبي' },
];

test('V110-F86 a localized display name is mentioned by login, a case difference keeps the display', () => {
  assert.equal(mentionName('مستخدم', 'rtl_arabic'), 'rtl_arabic');
  assert.equal(mentionName('Alice', 'alice'), 'Alice');
});

test('V110-F86 @ matches the login and the display name', () => {
  assert.deepEqual(matchPeople('rtl', people), ['rtl_arabic']);
  assert.deepEqual(matchPeople('مست', people), ['rtl_arabic']);
  assert.deepEqual(matchPeople('ali', people), ['Alice']);
});

test('V110-F86 Tab on @name never offers an emote', () => {
  assert.deepEqual(tabNames('@al', ['ALO', 'AlienDance'], people), ['@Alice']);
  assert.deepEqual(tabNames('@rtl', ['RTLemote'], people), ['@rtl_arabic']);
  assert.ok(tabNames('al', ['ALO'], people).includes('ALO'));
  assert.deepEqual(tabNames('@', ['ALO'], people), []);
});

test('V111-41 people you mention most come first within the same kind of match', () => {
  const crowd: Person[] = [
    { name: 'Aaron', display: 'Aaron' },
    { name: 'Abby', display: 'Abby' },
    { name: 'Azure', display: 'Azure' },
    { name: 'xab', display: 'xab' },
  ];
  assert.deepEqual(matchPeople('a', crowd), ['Abby', 'Aaron', 'Azure', 'xab']);
  const used = mentionRank([{ login: 'azure' }, { login: 'aaron' }, { login: 'xab' }]);
  assert.deepEqual(matchPeople('a', crowd, 8, used), ['Azure', 'Aaron', 'Abby', 'xab']);
  // A prefix match still beats a mentioned name that only contains it.
  assert.deepEqual(matchPeople('ab', crowd, 8, used), ['Abby', 'xab']);
  assert.deepEqual(tabNames('@a', [], crowd, 30, used), ['@Azure', '@Aaron', '@Abby', '@xab']);
});
