import assert from 'node:assert/strict';
import { test } from 'node:test';

import { MAX_REASON, warnCommand, warnSpot } from './warnBox.ts';

test('M10 a warn needs a reason: empty or blank sends nothing', () => {
  assert.equal(warnCommand('viewer', ''), null);
  assert.equal(warnCommand('viewer', '   '), null);
  assert.equal(warnCommand('', 'stop'), null);
});

test('M10 the reason goes through /warn, trimmed and capped', () => {
  assert.equal(warnCommand('viewer', '  stop it '), '/warn viewer stop it');
  const long = warnCommand('viewer', 'x'.repeat(MAX_REASON + 20)) ?? '';
  assert.equal(long.length, '/warn viewer '.length + MAX_REASON);
});

test('M10 the box stays in the window', () => {
  const view = { width: 400, height: 300 };
  const box = { width: 200, height: 40 };
  assert.deepEqual(warnSpot({ left: 20, top: 50, bottom: 70 }, box, view), { left: 20, top: 74 });
  assert.equal(warnSpot({ left: 380, top: 50, bottom: 70 }, box, view).left, 196);
  assert.equal(warnSpot({ left: 20, top: 270, bottom: 290 }, box, view).top, 226);
});

test('V105 the box sits beside the button, centred on it, and flips left at the edge', () => {
  const view = { width: 400, height: 300 };
  const box = { width: 200, height: 40 };
  assert.deepEqual(warnSpot({ left: 20, top: 100, bottom: 120, right: 60 }, box, view), { left: 64, top: 90 });
  assert.equal(warnSpot({ left: 300, top: 100, bottom: 120, right: 340 }, box, view).left, 96);
  assert.equal(warnSpot({ left: 20, top: 0, bottom: 20, right: 60 }, box, view).top, 4);
});
