import assert from 'node:assert/strict';
import { test } from 'node:test';

import { tabSoundMode } from './tabSound.ts';

test('V106-25 a muted tab shows muted, playing or not', () => {
  assert.equal(tabSoundMode(true, true), 'muted');
  assert.equal(tabSoundMode(false, true), 'muted');
});

test('V106-25 an unmuted tab is audible only while something would be heard', () => {
  assert.equal(tabSoundMode(true, false), 'audible');
  assert.equal(tabSoundMode(false, false), 'silent');
});

test('V102-15 a held tab with every player muted shows crossed', () => {
  assert.equal(tabSoundMode(false, false, true), 'muted');
  assert.equal(tabSoundMode(false, false, false), 'silent');
  assert.equal(tabSoundMode(true, false, true), 'audible');
});
