import assert from 'node:assert/strict';
import { test } from 'node:test';

import { CLOSE_MS, createIntent, headingToward, inTriangle, OPEN_MS, type Timers } from './intent.ts';

/** A clock the test moves by hand. */
function clock() {
  let now = 0;
  let next = 1;
  const jobs = new Map<number, { at: number; fn: () => void }>();
  const timers: Timers = {
    set(fn, ms) {
      jobs.set(next, { at: now + ms, fn });
      return next++;
    },
    clear: (id) => void jobs.delete(id),
  };
  const advance = (ms: number) => {
    now += ms;
    for (const [id, j] of [...jobs]) {
      if (j.at <= now) {
        jobs.delete(id);
        j.fn();
      }
    }
  };
  return { timers, advance };
}

function setup() {
  const c = clock();
  const log: string[] = [];
  const intent = createIntent({ show: (d, i) => log.push(`show ${d}:${i}`), hide: (d) => log.push(`hide ${d}`) }, c.timers);
  return { ...c, intent, log };
}

test('V110-U69 a hovered submenu row opens after a short wait, not at once', () => {
  const t = setup();
  t.intent.enter({ depth: 0, index: 2, sub: true, open: null });
  t.advance(OPEN_MS - 1);
  assert.deepEqual(t.log, []);
  t.advance(1);
  assert.deepEqual(t.log, ['show 0:2']);
});

test('V110-U69 passing over a submenu row opens nothing', () => {
  const t = setup();
  t.intent.enter({ depth: 0, index: 2, sub: true, open: null });
  t.advance(60);
  t.intent.enter({ depth: 0, index: 3, sub: false, open: null });
  t.advance(1000);
  assert.deepEqual(t.log, []);
});

test('V110-U69 leaving a row for a plain one closes the flyout after the longer wait', () => {
  const t = setup();
  t.intent.enter({ depth: 0, index: 4, sub: false, open: 2 });
  t.advance(CLOSE_MS - 1);
  assert.deepEqual(t.log, []);
  t.advance(1);
  assert.deepEqual(t.log, ['hide 0']);
});

test('V110-U69 reaching the flyout in time keeps it open', () => {
  const t = setup();
  t.intent.enter({ depth: 0, index: 4, sub: false, open: 2 });
  t.advance(200);
  t.intent.panel();
  t.advance(1000);
  assert.deepEqual(t.log, []);
});

test('V110-U69 back on the row that owns the flyout cancels the close', () => {
  const t = setup();
  t.intent.enter({ depth: 0, index: 4, sub: false, open: 2 });
  t.advance(100);
  t.intent.enter({ depth: 0, index: 2, sub: true, open: 2 });
  t.advance(1000);
  assert.deepEqual(t.log, []);
  assert.equal(t.intent.pending, false);
});

test('V110-U69 another submenu row swaps the flyout; heading toward the old one waits longer', () => {
  const t = setup();
  t.intent.enter({ depth: 0, index: 3, sub: true, open: 2 });
  t.advance(OPEN_MS);
  assert.deepEqual(t.log, ['show 0:3']);
  t.intent.enter({ depth: 0, index: 4, sub: true, open: 3, toward: true });
  t.advance(OPEN_MS);
  assert.deepEqual(t.log, ['show 0:3']);
  t.advance(CLOSE_MS - OPEN_MS);
  assert.deepEqual(t.log, ['show 0:3', 'show 0:4']);
});

test('V110-U69 a typed field or armed item in a flyout keeps it open', () => {
  const t = setup();
  t.intent.enter({ depth: 0, index: 4, sub: false, open: 2, busy: true });
  t.advance(5000);
  assert.deepEqual(t.log, []);
});

test('V110-U69 a plain row with no flyout open schedules nothing', () => {
  const t = setup();
  t.intent.enter({ depth: 0, index: 1, sub: false, open: null });
  assert.equal(t.intent.pending, false);
});

test('V110-U69 cancel drops the pending open', () => {
  const t = setup();
  t.intent.enter({ depth: 1, index: 0, sub: true, open: null });
  t.intent.cancel();
  t.advance(1000);
  assert.deepEqual(t.log, []);
});

test('V110-U69 safe triangle: toward the flyout is inside, away or sideways is not', () => {
  assert.equal(inTriangle({ x: 1, y: 1 }, { x: 0, y: 0 }, { x: 4, y: 0 }, { x: 0, y: 4 }), true);
  assert.equal(inTriangle({ x: 5, y: 5 }, { x: 0, y: 0 }, { x: 4, y: 0 }, { x: 0, y: 4 }), false);
  // Parent's right edge at x=200; the flyout stands at x=204..404, y=100..300.
  const child = { left: 204, top: 100, right: 404, bottom: 300 };
  const from = { x: 150, y: 140 };
  assert.equal(headingToward(from, { x: 190, y: 170 }, child, 'right'), true);
  assert.equal(headingToward(from, { x: 190, y: 240 }, child, 'right'), true);
  assert.equal(headingToward(from, { x: 150, y: 320 }, child, 'right'), false);
  assert.equal(headingToward(from, { x: 100, y: 140 }, child, 'right'), false);
});

test('V110-U69 safe triangle mirrors for a flyout on the left', () => {
  const child = { left: -204, top: 100, right: -4, bottom: 300 };
  const from = { x: 50, y: 140 };
  assert.equal(headingToward(from, { x: 10, y: 170 }, child, 'left'), true);
  assert.equal(headingToward(from, { x: 90, y: 170 }, child, 'left'), false);
});
