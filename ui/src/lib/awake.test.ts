import assert from 'node:assert/strict';
import { type TestContext, test } from 'node:test';

import { everyShown, onShown } from './awake.ts';

class FakePage extends EventTarget {
  hidden = false;
  set(hidden: boolean) {
    this.hidden = hidden;
    this.dispatchEvent(new Event('visibilitychange'));
  }
}

function setup(t: TestContext) {
  t.mock.timers.enable({ apis: ['setInterval'] });
  const doc = new FakePage();
  let runs = 0;
  const stop = everyShown(() => runs++, 1000, doc);
  return { doc, runs: () => runs, stop };
}

test('shown: runs every tick as before (V107-H1)', (t) => {
  const { runs } = setup(t);
  t.mock.timers.tick(3000);
  assert.equal(runs(), 3);
});

test('hidden: skips its ticks, catches up once when shown', (t) => {
  const { doc, runs } = setup(t);
  doc.set(true);
  t.mock.timers.tick(5000);
  assert.equal(runs(), 0, 'nothing while hidden');
  doc.set(false);
  assert.equal(runs(), 1, 'one catch-up');
  doc.set(false);
  assert.equal(runs(), 1, 'only once');
  t.mock.timers.tick(1000);
  assert.equal(runs(), 2, 'back to the usual ticks');
});

test('hidden and back between ticks: no extra run', (t) => {
  const { doc, runs } = setup(t);
  doc.set(true);
  doc.set(false);
  assert.equal(runs(), 0);
});

test('stopped: no ticks, no catch-up', (t) => {
  const { doc, runs, stop } = setup(t);
  doc.set(true);
  t.mock.timers.tick(2000);
  stop();
  doc.set(false);
  t.mock.timers.tick(2000);
  assert.equal(runs(), 0);
});

test('onShown reports each change', () => {
  const doc = new FakePage();
  const seen: boolean[] = [];
  const stop = onShown((h) => seen.push(h), doc);
  doc.set(true);
  doc.set(false);
  stop();
  doc.set(true);
  assert.deepEqual(seen, [true, false]);
});
