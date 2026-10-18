import assert from 'node:assert/strict';
import { test } from 'node:test';

import { dismissWatch } from './dismiss.ts';

test('no events: not dismissed', () => {
  const t = new EventTarget();
  assert.equal(dismissWatch(t)(), false);
});

test('a press dismisses', () => {
  const t = new EventTarget();
  const stop = dismissWatch(t);
  t.dispatchEvent(new Event('pointerdown'));
  assert.equal(stop(), true);
});

test('only Escape among keys', () => {
  const key = (k: string) => Object.assign(new Event('keydown'), { key: k });
  const a = new EventTarget();
  const sa = dismissWatch(a);
  a.dispatchEvent(key('a'));
  assert.equal(sa(), false);
  const b = new EventTarget();
  const sb = dismissWatch(b);
  b.dispatchEvent(key('Escape'));
  assert.equal(sb(), true);
});

test('after stop the listeners are gone', () => {
  const t = new EventTarget();
  const first = dismissWatch(t);
  first();
  const second = dismissWatch(t);
  t.dispatchEvent(new Event('wheel'));
  assert.equal(second(), true);
  const third = dismissWatch(t);
  assert.equal(third(), false);
});
