import assert from 'node:assert/strict';
import { test } from 'node:test';

import { splitNames } from './noticeNames.ts';

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
