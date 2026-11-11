import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { chatHiddenIn, chatToggle, escLeaves, headerHidden, stillHere, theatreTarget, toggled } from './theatre.ts';

const tiles = [
  { id: 'a', player: false },
  { id: 'b', player: true },
  { id: 'c', player: true },
];

test('V108-N14 the heard stream goes to theatre, else the first with a stream', () => {
  assert.equal(theatreTarget(tiles, 'c'), 'c');
  assert.equal(theatreTarget(tiles, null), 'b');
  assert.equal(theatreTarget(tiles, 'a'), 'b');
  assert.equal(theatreTarget([{ id: 'a', player: false }], null), null);
});

test('V108-N14 the hotkey enters and leaves', () => {
  assert.equal(toggled(null, 'b'), 'b');
  assert.equal(toggled('b', 'c'), null);
  assert.equal(toggled(null, null), null);
});

test('V108-N14 theatre ends with its tile or tab', () => {
  assert.equal(stillHere('b', ['b', 'c']), 'b');
  assert.equal(stillHere('b', ['c']), null);
  assert.equal(stillHere(null, ['c']), null);
});

const esc = { key: 'Escape', composing: false, taken: false, fullscreen: false, sinceFullscreen: 5000 };

test('V108-N14 Esc leaves theatre', () => {
  assert.equal(escLeaves(esc, 'b'), true);
  assert.equal(escLeaves(esc, null), false);
  assert.equal(escLeaves({ ...esc, key: 'a' }, 'b'), false);
});

test('V108-N14 Esc in fullscreen, in a menu or right after fullscreen ended stays out of theatre', () => {
  assert.equal(escLeaves({ ...esc, fullscreen: true }, 'b'), false);
  assert.equal(escLeaves({ ...esc, taken: true }, 'b'), false);
  assert.equal(escLeaves({ ...esc, sinceFullscreen: 50 }, 'b'), false);
  assert.equal(escLeaves({ ...esc, composing: true }, 'b'), false);
});

test('V110-B2 the stream page header hides for the tile in theatre, unless it carries the buttons', () => {
  assert.equal(headerHidden('a', 'a', true), true);
  assert.equal(headerHidden('a', 'b', true), false);
  assert.equal(headerHidden(null, 'a', true), false);
  assert.equal(headerHidden('a', 'a', false), false);
  const src = readFileSync(new URL('../channel/StreamSide.svelte', import.meta.url), 'utf8');
  assert.match(src, /headerHidden\(theatre\.tile, tile\.id, !actions\)/);
});

test('V110-B10 the chat button exists only in theatre, for its tile, and says what a click does', () => {
  assert.equal(chatToggle(null, 'a', true, false), null);
  assert.equal(chatToggle('b', 'a', true, false), null);
  assert.equal(chatToggle('a', 'a', false, false), null);
  assert.equal(chatToggle('a', 'a', true, false), 'Hide chat');
  assert.equal(chatToggle('a', 'a', true, true), 'Show chat');
});

test('V110-B10 a hidden chat is hidden only in the theatre tile', () => {
  assert.equal(chatHiddenIn('a', 'a', true), true);
  assert.equal(chatHiddenIn('a', 'a', false), false);
  assert.equal(chatHiddenIn('a', 'b', true), false);
  assert.equal(chatHiddenIn(null, 'a', true), false);
});
