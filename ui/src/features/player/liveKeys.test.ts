import assert from 'node:assert/strict';
import { test } from 'node:test';

import { FocusOrigin } from './focusOrigin.ts';
import { liveKeyAction, OWN_ARROWS, ownsKey, TAB_ANY, seekText, stepChoice, Steps, stepText, behindText } from './liveKeys.ts';

/** qd-1 A DOM-like element: a tag, attributes and a parent; `closest` walks the real ancestor
 *  chain and matches the selector subset used here (tag, [a], [a="v"], [a^="v"], :not([...])). */
type El = { tag: string; attrs: Record<string, string>; parent: El | null; closest(sel: string): El | null };
function el(tag: string, attrs: Record<string, string> = {}, parent: El | null = null): El {
  const me: El = {
    tag,
    attrs,
    parent,
    closest(sel) {
      for (let n: El | null = me; n; n = n.parent) if (sel.split(',').some((p) => matches(n!, p.trim()))) return n;
      return null;
    },
  };
  return me;
}
function attrOk(n: El, a: string): boolean {
  const m = /^\[([\w-]+)(?:(\^?=)"([^"]*)")?\]$/.exec(a);
  if (!m) throw new Error(`selector not modeled: ${a}`);
  const v = n.attrs[m[1]];
  if (v === undefined) return false;
  return !m[2] || (m[2] === '=' ? v === m[3] : v.startsWith(m[3]));
}
function matches(n: El, sel: string): boolean {
  const m = /^([a-z]*)((?:\[[^\]]+\])*)((?::not\(\[[^\]]+\]\))*)$/.exec(sel);
  if (!m) throw new Error(`selector not modeled: ${sel}`);
  if (m[1] && m[1] !== n.tag) return false;
  for (const a of m[2].match(/\[[^\]]+\]/g) ?? []) if (!attrOk(n, a)) return false;
  for (const a of m[3].match(/\[[^\]]+\]/g) ?? []) if (attrOk(n, a)) return false;
  return true;
}
const key = (k: string, shiftKey = false) => ({ key: k, shiftKey });

test('which focused elements keep the arrows', () => {
  const kept = [el('input'), el('textarea'), el('div', { role: 'slider', tabindex: '0' }), el('div', { role: 'grid' }), el('div', { role: 'log' }), el('div', { 'data-arrows': '' })];
  for (const n of kept) assert.equal(ownsKey(n, key('ArrowLeft')), true, JSON.stringify(n.attrs) + n.tag);
  assert.equal(ownsKey(el('div'), key('ArrowLeft')), false);
  assert.equal(ownsKey(null, key('ArrowLeft')), false);
});

test('d-2 J/L and Shift+Right work anywhere but in a text field', () => {
  const tab = el('div', { role: 'tab' }, el('div', { role: 'tablist' }));
  const slider = el('div', { role: 'slider', tabindex: '0' });
  for (const k of [key('j'), key('l'), key('ArrowRight', true)]) {
    assert.equal(ownsKey(tab, k, () => true), false);
    assert.equal(ownsKey(slider, k), false);
    assert.equal(ownsKey(el('input'), k), true);
    assert.equal(ownsKey(el('div', { contenteditable: 'true' }), k), true);
    assert.equal(ownsKey(el('div', { contenteditable: 'false' }), k), false);
  }
});

test('d-2 a tab keeps only the arrows of its own bar, and only with keyboard focus', () => {
  assert.ok(!OWN_ARROWS.split(', ').includes(TAB_ANY));
  const yes = () => true;
  // rd-1 A clicked tab: the focus came from the pointer, the arrows seek.
  const across = el('div', { role: 'tab' });
  assert.equal(ownsKey(across, key('ArrowLeft'), () => false), false);
  assert.equal(ownsKey(across, key('ArrowLeft'), yes), true);
  assert.equal(ownsKey(across, key('ArrowRight'), yes), true);
  assert.equal(ownsKey(across, key('ArrowUp'), yes), false);
  const down = el('div', { role: 'tab', 'data-vertical': '1' });
  assert.equal(ownsKey(down, key('ArrowDown'), yes), true);
  assert.equal(ownsKey(down, key('ArrowLeft'), yes), false);
  assert.equal(ownsKey(el('div'), key('ArrowLeft'), yes), false);
});

test('qd-1 a tab inside its tablist: a click seeks, keyboard focus moves tabs', () => {
  const bar = el('div', { role: 'tablist', 'data-tab-bar': '' }, el('main'));
  const tab = el('div', { role: 'tab', tabindex: '0' }, bar);
  const vtab = el('div', { role: 'tab', tabindex: '0', 'data-vertical': '1' }, el('div', { role: 'tablist' }));
  assert.equal(ownsKey(tab, key('ArrowLeft'), () => false), false);
  assert.equal(ownsKey(tab, key('ArrowRight'), () => false), false);
  assert.equal(ownsKey(tab, key('ArrowLeft'), (t) => t === tab), true);
  assert.equal(ownsKey(tab, key('ArrowUp'), () => true), false);
  assert.equal(ownsKey(vtab, key('ArrowDown'), (t) => t === vtab), true);
  assert.equal(ownsKey(vtab, key('ArrowLeft'), () => true), false);
  // The bar itself (no tab focused) and the rest of OWN_ARROWS still keep them.
  assert.equal(ownsKey(bar, key('ArrowLeft'), () => false), true);
  assert.equal(ownsKey(el('div', { role: 'slider', tabindex: '0' }), key('ArrowLeft')), true);
  assert.equal(ownsKey(el('div', { role: 'slider', tabindex: '-1' }), key('ArrowLeft')), false);
  assert.equal(ownsKey(el('button', {}, el('div', { 'data-arrows': '' })), key('ArrowLeft')), true);
  assert.equal(ownsKey(el('button', {}, el('main')), key('ArrowLeft')), false);
  assert.equal(ownsKey(el('input', {}, tab), key('l')), true);
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
