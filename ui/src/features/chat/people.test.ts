import assert from 'node:assert/strict';
import { test } from 'node:test';

import { matchPeople, mentionName, tabNames, type Person } from './people.ts';

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
