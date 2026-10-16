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
