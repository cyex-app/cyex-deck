import assert from 'node:assert/strict';
import { test } from 'node:test';

import { fullscreenWatch, restorePlan, sizeButton, UNSEEN, wantFullscreen } from './theatreWindow.ts';

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

test('V110-B10 a window that stops being fullscreen ends theatre', () => {
  let w = fullscreenWatch(UNSEEN, { theatre: true, fullscreen: true, minimized: false });
  assert.deepEqual(w, { next: { seen: true }, leave: false });
  w = fullscreenWatch(w.next, { theatre: true, fullscreen: false, minimized: false });
  assert.equal(w.leave, true);
});

test('V110-B10 before the window went fullscreen, or while minimized, theatre stays', () => {
  assert.equal(fullscreenWatch(UNSEEN, { theatre: true, fullscreen: false, minimized: false }).leave, false);
  assert.equal(fullscreenWatch({ seen: true }, { theatre: true, fullscreen: false, minimized: true }).leave, false);
  assert.deepEqual(fullscreenWatch({ seen: true }, { theatre: true, fullscreen: false, minimized: true }).next, { seen: true });
  assert.equal(fullscreenWatch({ seen: true }, { theatre: false, fullscreen: false, minimized: false }).leave, false);
});

test('V110-B10 Restore in theatre leaves theatre', () => {
  assert.deepEqual(sizeButton(true, false), { icon: 'restore', label: 'Restore', leavesTheatre: true });
  assert.deepEqual(sizeButton(true, true), { icon: 'restore', label: 'Restore', leavesTheatre: true });
  assert.deepEqual(sizeButton(false, true), { icon: 'restore', label: 'Restore', leavesTheatre: false });
  assert.deepEqual(sizeButton(false, false), { icon: 'maximize', label: 'Maximize', leavesTheatre: false });
});
