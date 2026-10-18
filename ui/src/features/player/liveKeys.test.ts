import assert from 'node:assert/strict';
import { test } from 'node:test';

import { FocusOrigin } from './focusOrigin.ts';
import { liveKeyAction, OWN_ARROWS, ownsKey, TAB_ACROSS, TAB_DOWN, seekText, stepChoice, Steps, stepText, behindText } from './liveKeys.ts';

/** Matches when one of the selector's comma parts is in `parts`. */
const fake = (parts: string[]) => ({ closest: (sel: string) => (sel.split(', ').some((p) => parts.includes(p)) ? {} : null) });
const key = (k: string, shiftKey = false) => ({ key: k, shiftKey });

test('which focused elements keep the arrows', () => {
  for (const r of ['input', 'textarea', '[role="slider"]:not([tabindex="-1"])', '[role="grid"]', '[role="log"]', '[data-arrows]']) {
    assert.equal(ownsKey(fake([r]), key('ArrowLeft')), true, r);
  }
  assert.equal(ownsKey(fake([]), key('ArrowLeft')), false);
  assert.equal(ownsKey(null, key('ArrowLeft')), false);
});

test('d-2 J/L and Shift+Right work anywhere but in a text field', () => {
  const tab = fake([TAB_ACROSS]);
  const slider = fake(['[role="slider"]:not([tabindex="-1"])']);
  for (const k of [key('j'), key('l'), key('ArrowRight', true)]) {
    assert.equal(ownsKey(tab, k), false);
    assert.equal(ownsKey(slider, k), false);
    assert.equal(ownsKey(fake(['input']), k), true);
    assert.equal(ownsKey(fake(['[contenteditable]:not([contenteditable="false"])']), k), true);
  }
});

test('d-2 a tab keeps only the arrows of its own bar, and only with keyboard focus', () => {
  assert.ok(!OWN_ARROWS.split(', ').includes('[role="tab"]'));
  const yes = () => true;
  // rd-1 A clicked tab: the focus came from the pointer, the arrows seek.
  const across = fake([TAB_ACROSS]);
  assert.equal(ownsKey(across, key('ArrowLeft'), () => false), false);
  assert.equal(ownsKey(across, key('ArrowLeft'), yes), true);
  assert.equal(ownsKey(across, key('ArrowRight'), yes), true);
  assert.equal(ownsKey(across, key('ArrowUp'), yes), false);
  const down = fake([TAB_DOWN]);
  assert.equal(ownsKey(down, key('ArrowDown'), yes), true);
  assert.equal(ownsKey(down, key('ArrowLeft'), yes), false);
  assert.equal(ownsKey(fake([]), key('ArrowLeft'), yes), false);
});

test('rd-1 focus origin: a click, then keys, is still a click; Tab or arrows focus by key', () => {
  const o = new FocusOrigin();
  const tab = {};
  o.pointer();
  o.focus(tab);
  o.key();
  assert.equal(o.byKey(tab), false);
  const next = {};
  o.focus(next);
  assert.equal(o.byKey(next), true);
  assert.equal(o.byKey(tab), false);
  o.focus(tab);
  assert.equal(o.byKey(tab), true);
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
