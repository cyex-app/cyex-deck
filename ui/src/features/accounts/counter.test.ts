import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ConnectionCount } from '../../ipc/bindings/ConnectionCount';
import { counterView } from './counter.ts';

const name = (id: string) => (id === 'a' ? 'Bot' : 'Main');
const count = (userId: string, joined: number, refused: string[] = []): ConnectionCount => ({ userId, joined, refused });

test('one account shows its open chats', () => {
  const v = counterView([count('a', 12)], 'a', name);
  assert.deepEqual(v, { text: '12', warn: false, hidden: false, label: 'Joined chats', aria: '12 joined chats', hint: undefined });
});

test('several accounts: the number is the current one, the label lists all', () => {
  const v = counterView([count('a', 12), count('b', 3)], 'b', name);
  assert.equal(v.text, '3');
  assert.equal(v.label, 'Joined chats');
  assert.equal(v.hint, 'Bot 12, Main 3');
  assert.equal(v.warn, false);
});

test('nothing joined yet hides the counter', () => {
  assert.equal(counterView([], 'a', name).hidden, true);
  assert.equal(counterView([count('a', 0)], 'a', name).hidden, true);
  assert.equal(counterView([count('a', 1)], 'a', name).hidden, false);
});

test('numbers are grouped with dots', () => {
  assert.equal(counterView([count('a', 1234)], 'a', name).text, '1.234');
});

test('refused chats of the current account warn and name the channels', () => {
  const v = counterView([count('a', 5, ['a'])], 'a', name);
  assert.equal(v.warn, true);
  assert.equal(v.label, 'Too many chats joined');
  assert.equal(v.hint, '#a: not joined. Close a few chats, then open these again.');
});

test('another account is named', () => {
  const v = counterView([count('a', 5), count('b', 9, ['a', 'b'])], 'a', name);
  assert.equal(v.hint, 'Main: #a, #b: not joined. Close a few chats, then open these again.');
});

test('refusals on both accounts share one label', () => {
  const v = counterView([count('a', 5, ['x']), count('b', 9, ['y'])], 'a', name);
  assert.equal(v.label, 'Too many chats joined');
  assert.equal(v.hint, '#x; Main: #y: not joined. Close a few chats, then open these again.');
});

test('a long refused list is cut after five channels', () => {
  const refused = ['a', 'b', 'c', 'd', 'e', 'f', 'g'];
  const v = counterView([{ userId: '1', joined: 3, refused }], '1', () => 'x');
  assert.match(v.hint ?? '', /#a, #b, #c, #d, #e and 2 more: not joined/);
  assert.doesNotMatch(v.hint ?? '', /#f/);
});
