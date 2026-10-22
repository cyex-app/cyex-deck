import assert from 'node:assert/strict';
import { test } from 'node:test';

import { has, leaf, move, remove, split } from '../../lib/panels.ts';
import { closedOut, dockBack, entriesOf, keepOut, mineOut, panelRoute, parseRoute, poppedFor, poppedKey, settle, shownTree, withPopped } from './popped.ts';

const LAYOUT = split(
  'row',
  [leaf('chat'), split('column', [leaf('activity'), leaf('unban'), leaf('actions')], [0.5, 0.2, 0.3]), leaf('mass')],
  [0.4, 0.35, 0.25],
);

test('V107-R29 routes round-trip', () => {
  assert.equal(panelRoute('xqc', 'mass'), 'modpanel/xqc/mass');
  assert.deepEqual(parseRoute('modpanel/xqc/mass'), { login: 'xqc', id: 'mass' });
  assert.equal(parseRoute('modview/xqc'), null);
  assert.equal(parseRoute('modpanel/xqc'), null);
  assert.equal(poppedKey('xqc', 'mass'), 'xqc/mass');
});

test('V107-R29 only this channel’s pop-outs count', () => {
  const out = poppedFor(['modpanel/xqc/mass', 'modpanel/other/actions', 'modview/xqc', 'tab/1'], 'xqc');
  assert.deepEqual([...out], ['mass']);
});

test('V107-R29 a popped-out panel doesn’t show', () => {
  assert.ok(!has(shownTree(LAYOUT, new Set(['mass'])), 'mass'));
  assert.equal(shownTree(LAYOUT, new Set()), LAYOUT);
});

test('V107-R29 out of the layout and back, nothing changed: its exact place and size', () => {
  const back = dockBack(remove(LAYOUT, 'unban'), 'unban', LAYOUT, new Set());
  assert.ok(back.exact);
  assert.deepEqual(back.tree, LAYOUT);
});

test('V107-R29 exact also from a saved copy with other key order and rounding', () => {
  const saved = JSON.parse(JSON.stringify(remove(LAYOUT, 'unban')), (_k, v) => (typeof v === 'number' ? Math.round(v * 1e6) / 1e6 : v));
  assert.ok(dockBack(saved, 'unban', LAYOUT, new Set()).exact);
});

test('V107-R29 exact while another panel is still out', () => {
  const now = remove(remove(LAYOUT, 'unban'), 'mass');
  const back = dockBack(now, 'unban', LAYOUT, new Set(['mass']));
  assert.ok(back.exact);
  assert.deepEqual(back.tree, remove(LAYOUT, 'mass'));
});

test('V107-R29 the layout changed meanwhile: next to the panel it sat beside', () => {
  const now = move(remove(LAYOUT, 'unban'), 'mass', 'chat', 'bottom');
  const back = dockBack(now, 'unban', LAYOUT, new Set());
  assert.ok(!back.exact);
  assert.ok(has(back.tree, 'unban') && has(back.tree, 'mass'));
});

test('V107-R29 only closed windows dock back; panels from the closed row have no entry', () => {
  const popped = { 'xqc/unban': LAYOUT, 'xqc/mass': LAYOUT };
  assert.deepEqual(closedOut(popped, ['modpanel/xqc/mass']), ['xqc/unban']);
  assert.deepEqual(closedOut({}, []), []);
});

test('V107-N3e two panels out and back, in either order: the layout as it was', () => {
  const full = split('row', [leaf('actions'), leaf('rewards'), leaf('mass')], [0.25, 0.25, 0.5]);
  // `actions` out, then `rewards`: what `rewards` keeps is the whole layout.
  const afterA = remove(full, 'actions')!;
  const wasB = withPopped(afterA, [['actions', full]])!;
  assert.deepEqual(wasB, full);
  const afterB = remove(afterA, 'rewards')!;
  // actions back first, then rewards
  let t = dockBack(afterB, 'actions', full, new Set(['rewards'])).tree;
  let b = dockBack(t, 'rewards', wasB, new Set());
  assert.ok(b.exact);
  assert.deepEqual(b.tree, full);
  // rewards back first, then actions
  t = dockBack(afterB, 'rewards', wasB, new Set(['actions'])).tree;
  b = dockBack(t, 'actions', full, new Set());
  assert.ok(b.exact);
  assert.deepEqual(b.tree, full);
});

test('V107-N3e nothing else out: the layout itself', () => {
  assert.equal(withPopped(LAYOUT, []), LAYOUT);
});

test('V107-Mg pop-outs are per channel: the shared layout keeps the panel', () => {
  // xqc pops `mass` out: only xqc's view leaves it out.
  const out = poppedFor(['modpanel/xqc/mass'], 'xqc');
  assert.ok(!has(shownTree(LAYOUT, out), 'mass'));
  assert.ok(has(shownTree(LAYOUT, poppedFor(['modpanel/xqc/mass'], 'other')), 'mass'));
});

test('V107-Mg an edit in the channel with the pop-out keeps it in its spot', () => {
  const out = new Set(['unban']);
  const view = shownTree(LAYOUT, out);
  assert.deepEqual(mineOut(LAYOUT, out), ['unban']);
  // Unchanged view: exactly the layout.
  assert.deepEqual(keepOut(view, mineOut(LAYOUT, out), LAYOUT), LAYOUT);
  // A move elsewhere: `unban` still beside `activity`.
  const moved = keepOut(move(view, 'mass', 'chat', 'bottom'), ['unban'], LAYOUT)!;
  assert.ok(has(moved, 'unban') && has(moved, 'mass'));
  // Every panel left closed: just the one out.
  assert.deepEqual(keepOut(null, ['unban'], LAYOUT), leaf('unban'));
  assert.equal(keepOut(view, [], LAYOUT), view);
});

test('V107-Mg two out in one channel, an edit there: both back exactly', () => {
  const out = new Set(['unban', 'mass']);
  const back = keepOut(shownTree(LAYOUT, out), mineOut(LAYOUT, out), LAYOUT);
  assert.deepEqual(back, LAYOUT);
});

test('V107-Mg settle: closed windows dropped, panel already in the layout untouched', () => {
  const popped = { 'xqc/unban': LAYOUT, 'other/mass': LAYOUT };
  const s = settle(LAYOUT, popped, ['modpanel/other/mass'], new Set());
  assert.equal(s.tree, null);
  assert.deepEqual(s.drop, ['xqc/unban']);
  // About to open: kept.
  assert.deepEqual(settle(LAYOUT, popped, [], new Set(['xqc/unban', 'other/mass'])).drop, []);
});

test('V107-Mg settle: an old global pop-out goes back into the shared layout, exactly', () => {
  // Saved before: popping out removed `unban` from the layout every channel shares.
  const old = remove(LAYOUT, 'unban');
  const s = settle(old, { 'xqc/unban': LAYOUT }, ['modpanel/xqc/unban'], new Set());
  assert.deepEqual(s.tree, LAYOUT);
  assert.deepEqual(s.drop, []);
  // Two old ones out (each kept the whole layout, V107-N3e): both exactly.
  const two = remove(old, 'mass');
  assert.deepEqual(settle(two, { 'xqc/unban': LAYOUT, 'xqc/mass': LAYOUT }, [], new Set()).tree, LAYOUT);
});

test('V107-Mg closed in one view: its pop-out entries elsewhere', () => {
  assert.deepEqual(entriesOf({ 'xqc/mass': LAYOUT, 'other/mass': LAYOUT, 'xqc/unban': LAYOUT }, 'mass'), ['xqc/mass', 'other/mass']);
});
