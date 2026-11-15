import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { chatHiddenIn, chatToggle, escLeaves, headerHidden, REST_MS, stillHere, theatreTarget, toggled } from './theatre.ts';

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

test('V110-B10 in theatre the chat button is for its tile only and says what a click does', () => {
  const on = { player: true, chat: true };
  assert.equal(chatToggle('b', 'a', on, false, false), null);
  assert.equal(chatToggle('a', 'a', on, true, false), null, 'chat in its own window');
  assert.equal(chatToggle('a', 'a', on, false, false), 'Hide chat');
  assert.equal(chatToggle('a', 'a', on, false, true), 'Show chat');
});

test('V111-26 the chat button shows in a normal view, as the tab menu offers Hide chat / Show chat', () => {
  assert.equal(chatToggle(null, 'a', { player: true, chat: true }, false, false), 'Hide chat');
  assert.equal(chatToggle(null, 'a', { player: true, chat: true }, false, true), 'Hide chat', 'a stale theatre fold does not count');
  assert.equal(chatToggle(null, 'a', { player: true, chat: false }, false, false), 'Show chat');
  assert.equal(chatToggle('a', 'a', { player: true, chat: false }, false, false), 'Show chat', 'hidden by the menu, then theatre');
  assert.equal(chatToggle(null, 'a', { player: false, chat: true }, false, false), null, 'chat-only tile has no player');
  assert.equal(chatToggle(null, 'a', { player: true, chat: true }, true, false), null);
  const src = readFileSync(new URL('../channel/StreamSide.svelte', import.meta.url), 'utf8');
  assert.ok(src.includes("op: 'show', tile: tile.id, show: { player: true, chat: !tile.show.chat }"), 'same op as the menu');
  assert.ok(src.includes('!popout'), 'not in a pop-out');
});

test('V110-B10 a hidden chat is hidden only in the theatre tile', () => {
  assert.equal(chatHiddenIn('a', 'a', true), true);
  assert.equal(chatHiddenIn('a', 'a', false), false);
  assert.equal(chatHiddenIn('a', 'b', true), false);
  assert.equal(chatHiddenIn(null, 'a', true), false);
});

test('V110-B11 the chat button fades after 1 s of rest and is square and see-through', () => {
  assert.equal(REST_MS, 1000);
  const src = readFileSync(new URL('../channel/StreamSide.svelte', import.meta.url), 'utf8');
  assert.ok(src.includes('setTimeout(() => (resting = true), REST_MS)'), 'rests after REST_MS');
  assert.ok(src.includes('.body:hover .chat-toggle:not(.resting)'), 'hidden while resting');
  assert.ok(src.includes('border-radius: var(--radius);'), 'square corners');
});
