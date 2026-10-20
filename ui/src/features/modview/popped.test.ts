import assert from 'node:assert/strict';
import { test } from 'node:test';

import { has, leaf, move, remove, split } from '../../lib/panels.ts';
import { closedOut, dockBack, panelRoute, parseRoute, poppedFor, poppedKey, shownTree } from './popped.ts';

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
