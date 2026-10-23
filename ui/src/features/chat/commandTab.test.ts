import assert from 'node:assert/strict';
import { test } from 'node:test';

import { commandAt, commandHint, commandMatches, tabTakes } from './commandTab.ts';
import { wordAt } from './completion.ts';

const at = (text: string, caret = text.length) => commandAt(text, wordAt(text, caret));

test('V1041-01 only the first word after a slash', () => {
  assert.equal(at('/ba'), 'ba');
  assert.equal(at('  /ba'), 'ba');
  assert.equal(at('/'), '');
  // The caret in the first word, text after it.
  assert.equal(at('/ban someone', 3), 'ba');
  // Not a command: plain text, a slash later on, the caret past the first word.
  assert.equal(at('hello'), null);
  assert.equal(at('hi /ba'), null);
  assert.equal(at('/ban some', 9), null);
  assert.equal(at('/ban ', 5), null);
  assert.equal(at(''), null);
  // F14 A dot doesn't complete.
  assert.equal(at('.ba'), null);
});

const NAMES = ['ban', 'unban', 'timeout', 'untimeout', 'user', 'usercard', 'unblock', 'me', 'mods', 'mod', 'hello'];

test('V1041-01 matches rank like emotes, the exact name first', () => {
  assert.deepEqual(commandMatches('ba', NAMES), ['/ban', '/unban']);
  assert.deepEqual(commandMatches('ban', NAMES), ['/ban', '/unban']);
  // Another case fixes it.
  assert.deepEqual(commandMatches('BAN', NAMES), ['/ban', '/unban']);
  // Similar names: shorter first.
  assert.deepEqual(commandMatches('use', NAMES), ['/user', '/usercard']);
  assert.deepEqual(commandMatches('user', NAMES), ['/user', '/usercard']);
  assert.deepEqual(commandMatches('un', NAMES).slice(0, 3), ['/unban', '/unblock', '/untimeout']);
  assert.deepEqual(commandMatches('mod', NAMES), ['/mod', '/mods']);
  // Your own commands are just names.
  assert.deepEqual(commandMatches('hel', NAMES), ['/hello']);
});

test('V1041-01 nothing typed or nothing found: no matches', () => {
  assert.deepEqual(commandMatches('', NAMES), []);
  assert.deepEqual(commandMatches('zzz', NAMES), []);
  assert.deepEqual(commandMatches('/ba', NAMES), []);
  assert.deepEqual(commandMatches('ba', []), []);
});

test('commandHint: first sentence, aliases, unknown', () => {
  assert.equal(commandHint('set'), 'Saves a value your commands read with $get(var,key).');
  assert.equal(commandHint('SO'), commandHint('shoutout'));
  assert.equal(commandHint('nope'), undefined);
});

test('Tab is only taken when there is something to complete', () => {
  assert.equal(tabTakes('', 0), false);
  assert.equal(tabTakes('hi ', 3), false);
  assert.equal(tabTakes('hi @fo', 6), true);
  assert.equal(tabTakes('/', 1), true);
});
