import assert from 'node:assert/strict';
import { test } from 'node:test';

import { calloutAbove, CONFIRM_MS, NOTCH, type Timers } from './confirmCallout.ts';
import { confirmer } from './menuConfirm.ts';

function fake(): Timers & { fire: () => void; live: number } {
  const runs = new Map<number, () => void>();
  let n = 0;
  return {
    set: (run, ms) => {
      assert.equal(ms, CONFIRM_MS);
      runs.set(++n, run);
      return n;
    },
    clear: (id) => void runs.delete(id as number),
    fire: () => {
      const all = [...runs.values()];
      runs.clear();
      all.forEach((r) => r());
    },
    get live() {
      return runs.size;
    },
  };
}

const unpin = { label: 'Unpin', confirm: 'Click to unpin' };
const del = { label: 'Delete message', confirm: 'Click to delete' };
const hide = { label: 'Hide for yourself' };

function setup() {
  const t = fake();
  const seen: (object | null)[] = [];
  const c = confirmer<object>((a) => seen.push(a), t);
  return { t, seen, c };
}

test('V107-P5 an item without confirm runs at once', () => {
  const { c, seen } = setup();
  assert.equal(c.press(hide, undefined), true);
  assert.deepEqual(seen, []);
});

test('V107-P5 the first pick arms, the second runs and lets go', () => {
  const { c, seen, t } = setup();
  assert.equal(c.press(del, del.confirm), false);
  assert.equal(c.armed, del);
  assert.equal(t.live, 1);
  assert.equal(c.press(del, del.confirm), true);
  assert.equal(c.armed, null);
  assert.equal(t.live, 0, 'the timer is gone');
  assert.deepEqual(seen, [del, null]);
});

test('V107-P5 it lets go after 3 s', () => {
  const { c, t } = setup();
  c.press(unpin, unpin.confirm);
  t.fire();
  assert.equal(c.armed, null);
  assert.equal(c.press(unpin, unpin.confirm), false, 'a pick after the timeout arms again');
});

test('V107-P5 moving to another item lets go; staying on it does not', () => {
  const { c } = setup();
  c.press(del, del.confirm);
  c.moveTo(del);
  assert.equal(c.armed, del);
  c.moveTo(unpin);
  assert.equal(c.armed, null);
});

test('V107-P5 picking another confirm item arms that one instead', () => {
  const { c, t } = setup();
  c.press(unpin, unpin.confirm);
  assert.equal(c.press(del, del.confirm), false);
  assert.equal(c.armed, del);
  assert.equal(t.live, 1, 'one timer, restarted');
});

test('V107-P5 picking a plain item while one is armed runs it and lets go', () => {
  const { c } = setup();
  c.press(del, del.confirm);
  assert.equal(c.press(hide, undefined), true);
  assert.equal(c.armed, null);
});

test('V107-P5 Escape lets go first, then has nothing left to cancel', () => {
  const { c, t } = setup();
  c.press(del, del.confirm);
  assert.equal(c.cancel(), true);
  assert.equal(t.live, 0);
  assert.equal(c.cancel(), false, 'so the next Escape closes the menu');
});

test('V107-P5 the callout sits over the item, 1 px into it, and drops under it at the top', () => {
  const item = { left: 100, right: 300, top: 200, bottom: 230 };
  const p = calloutAbove(item, 90, 24, 1000);
  assert.deepEqual(p, { left: 155, top: 177, notch: 45, below: false });
  const top = calloutAbove({ ...item, top: 10, bottom: 40 }, 90, 24, 1000);
  assert.equal(top.below, true);
  assert.equal(top.top, 39);
  assert.ok(NOTCH > 0);
});
