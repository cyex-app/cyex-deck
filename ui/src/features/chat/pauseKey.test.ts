import assert from 'node:assert/strict';
import { test } from 'node:test';

import { editable, pauseHeld, type KeyLike } from './pauseKey.ts';

const ev = (key: string, mods: Partial<KeyLike> & { altGraph?: boolean } = {}, type = 'keydown'): KeyLike => ({
  type,
  key,
  altKey: false,
  ctrlKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
  getModifierState: (k) => k === 'AltGraph' && !!mods.altGraph,
});

test('the modifier alone holds the pause', () => {
  assert.equal(pauseHeld('alt', ev('Alt', { altKey: true }), false), true);
  assert.equal(pauseHeld('ctrl', ev('Control', { ctrlKey: true }), false), true);
  assert.equal(pauseHeld('shift', ev('Shift', { shiftKey: true }), false), true);
  assert.equal(pauseHeld('none', ev('Alt', { altKey: true }), false), false);
});

test('keyup and any other key let go', () => {
  assert.equal(pauseHeld('alt', ev('Alt', { altKey: false }, 'keyup'), false), false);
  assert.equal(pauseHeld('shift', ev('A', { shiftKey: true }), false), false);
  assert.equal(pauseHeld('ctrl', ev('v', { ctrlKey: true }), false), false);
  assert.equal(pauseHeld('alt', ev('Tab', { altKey: true }), false), false);
});

test('AltGr never pauses (Windows sends it as Ctrl+Alt)', () => {
  // Chromium on Windows: a Control keydown, then AltGraph, both with ctrl and alt set.
  assert.equal(pauseHeld('ctrl', ev('Control', { ctrlKey: true, altKey: true }), false), false);
  assert.equal(pauseHeld('ctrl', ev('Control', { ctrlKey: true, altGraph: true }), false), false);
  assert.equal(pauseHeld('alt', ev('AltGraph', { ctrlKey: true, altKey: true, altGraph: true }), false), false);
  assert.equal(pauseHeld('alt', ev('Alt', { ctrlKey: true, altKey: true }), false), false);
  // The @ it types.
  assert.equal(pauseHeld('alt', ev('@', { ctrlKey: true, altKey: true, altGraph: true }), false), false);
});

test('in a text field only Alt alone pauses', () => {
  assert.equal(pauseHeld('alt', ev('Alt', { altKey: true }), true), true);
  assert.equal(pauseHeld('alt', ev('AltGraph', { ctrlKey: true, altKey: true, altGraph: true }), true), false);
  assert.equal(pauseHeld('alt', ev('Alt', { ctrlKey: true, altKey: true }), true), false);
  assert.equal(pauseHeld('shift', ev('Shift', { shiftKey: true }), true), false);
  assert.equal(pauseHeld('ctrl', ev('Control', { ctrlKey: true }), true), false);
  assert.equal(editable({ tagName: 'TEXTAREA' }), true);
  assert.equal(editable({ tagName: 'INPUT' }), true);
  assert.equal(editable({ tagName: 'DIV', isContentEditable: true }), true);
  assert.equal(editable({ tagName: 'DIV', isContentEditable: false }), false);
  assert.equal(editable(null), false);
});
