import assert from 'node:assert/strict';
import { test } from 'node:test';

import { restorePlan, wantFullscreen } from './theatreWindow.ts';

test('V110-B7 theatre keeps the window fullscreen when a player leaves its own', () => {
  assert.equal(wantFullscreen(true, true), true);
  assert.equal(wantFullscreen(false, true), true);
  assert.equal(wantFullscreen(true, false), true);
  assert.equal(wantFullscreen(false, false), false);
});

test('V110-B7 leaving theatre restores maximized or normal size', () => {
  assert.equal(restorePlan({ maximized: true }, false), 'maximize');
  assert.equal(restorePlan({ maximized: false }, true), 'unmaximize');
  assert.equal(restorePlan({ maximized: true }, true), null);
  assert.equal(restorePlan({ maximized: false }, false), null);
});
