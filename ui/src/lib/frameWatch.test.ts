import assert from 'node:assert/strict';
import { mock, test } from 'node:test';

import { PING_EVERY_MS, startFrameWatch } from './frameWatch.ts';

/** A page whose frames run when `painting`, and whose visibility can change. */
function fakePage() {
  const listeners: (() => void)[] = [];
  const frames: (() => void)[] = [];
  const page = {
    visibilityState: 'visible',
    painting: true,
    addEventListener: (_: string, f: () => void) => listeners.push(f),
    removeEventListener: () => {},
    setVisible(v: boolean) {
      page.visibilityState = v ? 'visible' : 'hidden';
      for (const f of listeners) f();
    },
    /** Runs the frames asked for, if the page paints. */
    draw() {
      if (!page.painting) return;
      for (const f of frames.splice(0)) f();
    },
  };
  Object.assign(globalThis, {
    document: page,
    requestAnimationFrame: (f: () => void) => frames.push(f),
    cancelAnimationFrame: () => frames.splice(0),
  });
  return page;
}

test('V110-F02 one ping a second while frames run, none while they stop or the page hides', () => {
  mock.timers.enable({ apis: ['setTimeout'] });
  const page = fakePage();
  const pings: boolean[] = [];
  const stop = startFrameWatch(async (v) => void pings.push(v));
  const second = () => {
    mock.timers.tick(PING_EVERY_MS);
    page.draw();
  };
  second();
  second();
  assert.deepEqual(pings, [true, true]);
  page.painting = false;
  second();
  second();
  assert.deepEqual(pings, [true, true]);
  page.painting = true;
  page.draw();
  assert.deepEqual(pings, [true, true, true]);
  page.setVisible(false);
  second();
  assert.deepEqual(pings, [true, true, true, false]);
  // Shown again: no ping until a frame ran.
  page.setVisible(true);
  assert.equal(pings.length, 4);
  second();
  assert.deepEqual(pings.slice(4), [true]);
  stop();
  mock.timers.reset();
});
