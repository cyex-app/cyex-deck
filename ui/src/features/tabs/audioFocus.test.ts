import assert from 'node:assert/strict';
import { test } from 'node:test';

import { claimsOnClick, heardTile, hushedByFocus, isControl, unmutesOnClaim } from './audioFocus.ts';

const two = { players: ['a', 'b'], claimed: null, enabled: true, theatre: null };

test('V108-N13 with no pick the first stream is heard', () => {
  assert.equal(heardTile(two), 'a');
  assert.equal(hushedByFocus('a', two), false);
  assert.equal(hushedByFocus('b', two), true);
});

test('V108-N13 with no pick the first tile on air is heard, else the first', () => {
  assert.equal(heardTile({ ...two, live: ['b'] }), 'b');
  assert.equal(hushedByFocus('a', { ...two, live: ['b'] }), true);
  assert.equal(heardTile({ ...two, live: [] }), 'a');
  assert.equal(heardTile({ ...two, claimed: 'a', live: ['b'] }), 'a');
});

test('V108-N13 a click moves the sound; a tile that went away falls back to the first', () => {
  assert.equal(heardTile({ ...two, claimed: 'b' }), 'b');
  assert.equal(heardTile({ ...two, claimed: 'gone' }), 'a');
});

test('V108-N13 off, or a single stream: nobody is hushed', () => {
  assert.equal(heardTile({ ...two, enabled: false }), null);
  assert.equal(hushedByFocus('b', { ...two, enabled: false }), false);
  assert.equal(heardTile({ ...two, players: ['a'] }), null);
});

test('V108-N13 a tile without a stream is never hushed', () => {
  assert.equal(hushedByFocus('chat-only', two), false);
});

test('V108-N14 the tile in theatre is heard, even with the focus off', () => {
  const t = { ...two, enabled: false, theatre: 'b' };
  assert.equal(heardTile(t), 'b');
  assert.equal(hushedByFocus('a', t), true);
  assert.equal(heardTile({ ...t, theatre: 'other-tab' }), null);
});

test('V108-N13 clicks in a text box leave the sound where it is', () => {
  assert.equal(claimsOnClick({ tagName: 'DIV' }), true);
  assert.equal(claimsOnClick({ tagName: 'textarea' }), false);
  assert.equal(claimsOnClick({ tagName: 'DIV', isContentEditable: true }), false);
  assert.equal(claimsOnClick(null), true);
});

test('V108-N13 a click that takes the sound unmutes a tile the user had muted', () => {
  const on = { silencedByFocus: true, tileMuted: true, onControl: false };
  assert.equal(unmutesOnClaim(on), true);
  // Already had the sound: its mute stays.
  assert.equal(unmutesOnClaim({ ...on, silencedByFocus: false }), false);
  // Not muted: nothing to undo.
  assert.equal(unmutesOnClaim({ ...on, tileMuted: false }), false);
  // On the player's own button or slider: that control decides.
  assert.equal(unmutesOnClaim({ ...on, onControl: true }), false);
});

test('V108-N13 buttons and sliders count as controls', () => {
  const hit = (s: string) => ({ closest: (q: string) => (q.includes(s) ? {} : null) });
  assert.equal(isControl(hit('button')), true);
  assert.equal(isControl(hit('[role="slider"]')), true);
  assert.equal(isControl({ closest: () => null }), false);
  assert.equal(isControl(null), false);
});
