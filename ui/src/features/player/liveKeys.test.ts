import assert from 'node:assert/strict';
import { test } from 'node:test';

import { liveKeyAction, OWN_ARROWS, ownsArrows, seekText, stepChoice, Steps, stepText, behindText } from './liveKeys.ts';

const fake = (roles: string[]) => ({ closest: (sel: string) => (roles.some((r) => sel.includes(r)) ? {} : null) });

test('which focused elements keep the arrows', () => {
  for (const r of ['input', 'textarea', '[role="slider"]', '[role="tab"]', '[role="grid"]', '[role="log"]', '[data-arrows]']) {
    assert.equal(ownsArrows(fake([r])), true, r);
  }
  assert.equal(ownsArrows(fake([])), false);
  assert.equal(ownsArrows(null), false);
});

test("a clicked seek bar (tabindex -1) doesn't keep the arrows", () => {
  const parts = OWN_ARROWS.split(", ");
  assert.ok(parts.includes('[role="slider"]:not([tabindex="-1"])'));
  assert.ok(!parts.includes('[role="slider"]'));
});

test('step and behind texts', () => {
  assert.equal(stepText(-10), '-10s');
  assert.equal(stepText(30), '+30s');
  assert.equal(stepText(-70), '-1:10');
  assert.equal(behindText(42), '-0:42');
  assert.equal(behindText(72), '-1:12');
  assert.equal(behindText(3723), '-1:02:03');
});

test('presses add up inside 800 ms', () => {
  const s = new Steps();
  s.add(-10, 0);
  s.add(-10, 300);
  assert.equal(s.add(-10, 600), -30);
  assert.equal(s.add(-10, 1500), -10);
});

test('step choice snaps to 5, 10, 15, 30', () => {
  assert.equal(stepChoice(7), 5);
  assert.equal(stepChoice(12), 10);
  assert.equal(stepChoice(99), 30);
});

test('seek readout', () => {
  assert.equal(seekText(null, -10), null);
  assert.equal(seekText(undefined, -20), '-20s');
  assert.equal(seekText({ behind: 'live', edge: null }, -10), 'Live');
  assert.equal(seekText({ behind: 0, edge: 'start' }, -10), 'Can’t go back further');
  assert.equal(seekText({ behind: 30, edge: null }, -10), '-0:30');
});

test('shift+right rules', () => {
  assert.equal(liveKeyAction({ repeat: true, owns: false, hasLive: true }), 'skip');
  assert.equal(liveKeyAction({ repeat: false, owns: true, hasLive: true }), 'pass');
  assert.equal(liveKeyAction({ repeat: false, owns: false, hasLive: false }), 'pass');
  assert.equal(liveKeyAction({ repeat: false, owns: false, hasLive: true }), 'run');
});
