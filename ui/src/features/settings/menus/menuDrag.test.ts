import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Node } from '../../../components/menus/types.ts';
import { group, item, type MenuDef, SEP } from '../../../components/menus/types.ts';
import { addPlace, applyDrop, applyMove, type Box, dropAt, type PanelBox, treeKey } from './menuDrag.ts';

const def: MenuDef<null> = {
  id: 'chat.name',
  label: 'Name',
  where: 'x',
  items: ['a', 'b', 'c', 'd', 'e', 'mine', 'plug'].map((id) => ({ id, name: id, build: () => null, slot: id === 'mine' || id === 'plug' ? (true as const) : undefined })),
  defaults: [item('a'), item('b'), SEP, group('g', 'More', [item('c'), item('d')]), item('e'), SEP, item('mine'), item('plug')],
};

function show(nodes: Node[]): string {
  return nodes.map((n) => (n.t === 'sep' ? '-' : n.t === 'group' ? `${n.id}[${show(n.children)}]` : n.id)).join(' ');
}

// Root panel at x 0..200, rows 30 high from y 0; the flyout at x 210..400.
const panels: PanelBox[] = [
  { id: null, left: 0, right: 200, top: 0, bottom: 240 },
  { id: 'g', left: 210, right: 400, top: 90, bottom: 150 },
];
const boxes: Box[] = [
  { key: 'a', panel: null, index: 0, kind: 'item', top: 0, bottom: 30 },
  { key: 'b', panel: null, index: 1, kind: 'item', top: 30, bottom: 60 },
  { key: 'sep:2', panel: null, index: 2, kind: 'sep', top: 60, bottom: 66 },
  { key: 'g', panel: null, index: 3, kind: 'group', top: 66, bottom: 96 },
  { key: 'e', panel: null, index: 4, kind: 'item', top: 96, bottom: 126 },
  { key: 'c', panel: 'g', index: 0, kind: 'item', top: 90, bottom: 120 },
  { key: 'd', panel: 'g', index: 1, kind: 'item', top: 120, bottom: 150 },
];
const item_ = { key: 'a', kind: 'item' as const };

test('a drop on the top half of a row lands before it, the bottom half after', () => {
  assert.deepEqual(dropAt(panels, boxes, 50, 36, item_), { t: 'line', panel: null, index: 1, key: 'b', edge: 'before' });
  assert.deepEqual(dropAt(panels, boxes, 50, 55, item_), { t: 'line', panel: null, index: 2, key: 'b', edge: 'after' });
});

test('the middle of a group row means into, its edges mean next to', () => {
  assert.deepEqual(dropAt(panels, boxes, 50, 81, item_), { t: 'into', group: 'g', key: 'g' });
  assert.equal(dropAt(panels, boxes, 50, 68, item_)?.t, 'line');
  assert.equal(dropAt(panels, boxes, 50, 94, item_)?.t, 'line');
});

test('a group never goes into a group, and not onto a flyout', () => {
  const g = { key: 'g', kind: 'group' as const };
  assert.equal(dropAt(panels, boxes, 50, 81, g)?.t, 'line');
  assert.equal(dropAt(panels, boxes, 300, 130, g), null);
});

test('past the ends of a panel lands first or last; outside lands nowhere', () => {
  assert.deepEqual(dropAt(panels, boxes, 300, 154, item_), { t: 'line', panel: 'g', index: 2, key: 'd', edge: 'after' });
  assert.deepEqual(dropAt(panels, boxes, 50, 200, item_), { t: 'line', panel: null, index: 5, key: 'e', edge: 'after' });
  assert.equal(dropAt(panels, boxes, 600, 50, item_), null);
});

test('an empty flyout takes a drop at its start', () => {
  const d = dropAt([{ id: 'g', left: 0, right: 100, top: 0, bottom: 30 }], [], 10, 10, item_);
  assert.deepEqual(d, { t: 'line', panel: 'g', index: 0, key: null, edge: 'before' });
});

test('applyDrop moves within the list, across it, into a group and out of a flyout', () => {
  const nodes = structuredClone(def.defaults);
  // a to after b: the line index 2 counts a itself.
  assert.equal(show(applyDrop(nodes, [0], { t: 'line', panel: null, index: 2, key: 'b', edge: 'after' })), 'b a - g[c d] e - mine plug');
  // onto itself: nothing.
  assert.equal(applyDrop(nodes, [0], { t: 'line', panel: null, index: 1, key: 'b', edge: 'before' }), nodes);
  assert.equal(show(applyDrop(nodes, [0], { t: 'into', group: 'g', key: 'g' })), 'b - g[c d a] e - mine plug');
  // d out of the flyout to the top before e.
  assert.equal(show(applyDrop(nodes, [3, 1], { t: 'line', panel: null, index: 4, key: 'e', edge: 'before' })), 'a b - g[c] d e - mine plug');
  // c within its flyout, after d.
  assert.equal(show(applyDrop(nodes, [3, 0], { t: 'line', panel: 'g', index: 2, key: 'd', edge: 'after' })), 'a b - g[d c] e - mine plug');
});

test('Add lands before the line that sets the slots apart, or at the end of the open group', () => {
  const nodes = structuredClone(def.defaults);
  assert.deepEqual(addPlace(def, nodes, null), { into: null, index: 5 });
  assert.deepEqual(addPlace(def, nodes, 'g'), { into: 'g', index: 2 });
  assert.deepEqual(addPlace(def, [item('a')], null), { into: null, index: 1 });
  assert.deepEqual(addPlace(def, [item('a')], 'gone'), { into: null, index: 1 });
});

test('keys: Alt+arrows move, plain arrows focus, F2 renames, Delete removes', () => {
  const row = { kind: 'item' as const, depth: 1 as const };
  assert.deepEqual(treeKey({ key: 'ArrowUp', altKey: true }, row), { t: 'nudge', dir: -1 });
  assert.deepEqual(treeKey({ key: 'ArrowDown', altKey: true }, row), { t: 'nudge', dir: 1 });
  assert.deepEqual(treeKey({ key: 'ArrowRight', altKey: true }, row), { t: 'indent' });
  assert.equal(treeKey({ key: 'ArrowLeft', altKey: true }, row), null);
  assert.deepEqual(treeKey({ key: 'ArrowLeft', altKey: true }, { kind: 'item', depth: 2 }), { t: 'outdent' });
  assert.equal(treeKey({ key: 'ArrowRight', altKey: true }, { kind: 'group', depth: 1 }), null);
  assert.deepEqual(treeKey({ key: 'ArrowDown' }, row), { t: 'focus', to: 'next' });
  assert.deepEqual(treeKey({ key: 'ArrowRight' }, { kind: 'group', depth: 1 }), { t: 'focus', to: 'child' });
  assert.deepEqual(treeKey({ key: 'ArrowLeft' }, { kind: 'item', depth: 2 }), { t: 'focus', to: 'parent' });
  assert.deepEqual(treeKey({ key: 'F2' }, row), { t: 'rename' });
  assert.equal(treeKey({ key: 'F2' }, { kind: 'sep', depth: 1 }), null);
  assert.deepEqual(treeKey({ key: 'Delete' }, row), { t: 'remove' });
  assert.equal(treeKey({ key: 'ArrowDown', ctrlKey: true }, row), null);
  assert.equal(treeKey({ key: 'x' }, row), null);
});

test('applyMove turns the move keys into tree edits', () => {
  const nodes = structuredClone(def.defaults);
  assert.equal(show(applyMove(nodes, [0], { t: 'nudge', dir: 1 })), 'b a - g[c d] e - mine plug');
  assert.equal(applyMove(nodes, [0], { t: 'nudge', dir: -1 }), nodes);
  assert.equal(show(applyMove(nodes, [4], { t: 'indent' })), 'a b - g[c d e] - mine plug');
  assert.equal(show(applyMove(nodes, [3, 0], { t: 'outdent' })), 'a b - g[d] c e - mine plug');
});
