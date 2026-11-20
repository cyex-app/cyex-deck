import assert from 'node:assert/strict';
import { test } from 'node:test';

import { argAt, argMatches, type ArgSpot } from './commandArgs.ts';

/** `|` marks the caret. */
const at = (marked: string) => argAt(marked.replace('|', ''), marked.indexOf('|'));

test('V1051-04 /combo, /paint and /badge arguments', () => {
  assert.deepEqual(at('/combo ni|'), { command: 'combo', sub: null, start: 7, typed: 'ni', quote: false });
  assert.deepEqual(at('/combo "night o|'), { command: 'combo', sub: null, start: 7, typed: 'night o', quote: true });
  assert.deepEqual(at('/combo delete g|'), { command: 'combo', sub: 'delete', start: 14, typed: 'g', quote: false });
  assert.deepEqual(at('/combo rename ni|'), { command: 'combo', sub: 'rename', start: 14, typed: 'ni', quote: false });
  assert.deepEqual(at('/combo delete “ni|'), { command: 'combo', sub: 'delete', start: 14, typed: 'ni', quote: true });
  assert.deepEqual(at('/paint Forest Sp|'), { command: 'paint', sub: null, start: 7, typed: 'Forest Sp', quote: false });
  assert.deepEqual(at('/PAINT x|'), { command: 'paint', sub: null, start: 7, typed: 'x', quote: false });
  assert.deepEqual(at('  /badge |'), { command: 'badge', sub: null, start: 9, typed: '', quote: false });
});

test('I106-3 /remove and /rename complete the first word', () => {
  assert.deepEqual(at('/remove KE|'), { command: 'remove', sub: null, start: 8, typed: 'KE', quote: false });
  assert.deepEqual(at('/REMOVE |'), { command: 'remove', sub: null, start: 8, typed: '', quote: false });
  assert.deepEqual(at('/rename KE|'), { command: 'rename', sub: null, start: 8, typed: 'KE', quote: false });
  assert.equal(at('/rename KEKW n|'), null);
  assert.equal(at('/add x|'), null);
  assert.deepEqual(argMatches({ command: 'remove', sub: null, start: 8, typed: 'ke', quote: false }, ['KEKW', 'Pog', 'my emote']), ['KEKW']);
  assert.deepEqual(argMatches({ command: 'rename', sub: null, start: 8, typed: '', quote: false }, ['a b']), ['a b']);
});

test('V111-03 # completes channels, names know the #channel', () => {
  assert.deepEqual(at('/add x #fo|'), { command: 'add', sub: '#', start: 8, typed: 'fo', quote: false });
  assert.deepEqual(at('/steal KEKW #|'), { command: 'steal', sub: '#', start: 13, typed: '', quote: false });
  assert.deepEqual(at('/remove a b #x|'), { command: 'remove', sub: '#', start: 13, typed: 'x', quote: false });
  // /remove: every name word; the #channel anywhere in the line goes along.
  assert.deepEqual(at('/remove a #Forsen K|'), { command: 'remove', sub: 'forsen', start: 18, typed: 'K', quote: false });
  assert.deepEqual(at('/remove a b|'), { command: 'remove', sub: null, start: 10, typed: 'b', quote: false });
  // The others: only the first name word.
  assert.deepEqual(at('/steal KE| #forsen'), { command: 'steal', sub: 'forsen', start: 7, typed: 'KE', quote: false });
  assert.deepEqual(at('/yoink #forsen KE|'), { command: 'yoink', sub: 'forsen', start: 15, typed: 'KE', quote: false });
  assert.deepEqual(at('/whoadded K|'), { command: 'whoadded', sub: null, start: 10, typed: 'K', quote: false });
  assert.equal(at('/steal KEKW n|'), null);
  assert.equal(at('/whoadded KEKW x|'), null);
  assert.equal(at('/add 01HZX|'), null);
  assert.deepEqual(argMatches({ command: 'add', sub: '#', start: 0, typed: 'fo', quote: false }, ['forsen', 'xqc']), ['forsen']);
});

test("V112-17 7TV's commands complete like cyex.tv's", () => {
  assert.deepEqual(at('/7add x #fo|'), { command: '7add', sub: '#', start: 9, typed: 'fo', quote: false });
  assert.deepEqual(at('/7remove a #Forsen K|'), { command: '7remove', sub: 'forsen', start: 19, typed: 'K', quote: false });
  assert.deepEqual(at('/7remove a b|'), { command: '7remove', sub: null, start: 11, typed: 'b', quote: false });
  assert.deepEqual(at('/7steal KE| #forsen'), { command: '7steal', sub: 'forsen', start: 8, typed: 'KE', quote: false });
  assert.deepEqual(at('/7whoadded K|'), { command: '7whoadded', sub: null, start: 11, typed: 'K', quote: false });
  assert.equal(at('/7add 01HZX|'), null);
  assert.equal(at('/7steal KEKW n|'), null);
  assert.equal(at('/77add x #f|'), null);
});

test('V1051-04 no argument to complete', () => {
  assert.equal(at('/combo save x|'), null);
  assert.equal(at('/combo rename "a b" c|'), null);
  assert.equal(at('/combo delete a b|'), null);
  assert.equal(at('/combo "a b"|'), null);
  assert.equal(at('hello /paint x|'), null);
  // The caret in the command name.
  assert.equal(at('/com|bo'), null);
  assert.equal(at('/combo|'), null);
});

const spot = (over: Partial<ArgSpot>): ArgSpot => ({ command: 'combo', sub: null, start: 0, typed: '', quote: false, ...over });

test('V1051-04 argMatches ranks like the helper and quotes names with spaces', () => {
  assert.deepEqual(argMatches(spot({ typed: 'for' }), ['Comfort', 'Forest Spring']), ['"Forest Spring"', 'Comfort']);
  assert.deepEqual(argMatches(spot({ typed: 'ni' }), ['Night Out']), ['"Night Out"']);
  assert.deepEqual(argMatches(spot({ typed: 'night o', quote: true }), ['Night Out']), ['"Night Out"']);
  assert.deepEqual(argMatches(spot({ typed: 'list' }), ['list', 'lunch']), ['list']);
  // V1051-05 Empty typed: combo names first, then the sub-commands.
  assert.deepEqual(argMatches(spot({}), ['list', 'save', 'Night Out', 'Beach']), ['"Night Out"', 'Beach', 'list', 'save']);
  assert.deepEqual(argMatches(spot({ sub: 'delete' }), ['Night Out', 'Beach']), ['"Night Out"', 'Beach']);
});

test('V1051-04 paint and badge names are never quoted', () => {
  assert.deepEqual(argMatches(spot({ command: 'paint', typed: 'Forest Sp' }), ['Forest Spring']), ['Forest Spring']);
  assert.deepEqual(argMatches(spot({ command: 'badge', quote: true, typed: 'Gol' }), ['Gold Star']), ['Gold Star']);
});
