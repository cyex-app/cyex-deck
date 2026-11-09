import assert from 'node:assert/strict';
import { test } from 'node:test';

import {
  addCustom,
  addGroup,
  addItem,
  addSep,
  indent,
  isDefault,
  move,
  moveInto,
  nodeAt,
  nudge,
  outOf,
  pathOf,
  remove,
  rename,
  reset,
  resetPatch,
  savePatch,
  setHidden,
  unused,
} from './ops.ts';
import { group, item, type MenuDef, type Node, SEP } from './types.ts';

const def: MenuDef<null> = {
  id: 'chat.name',
  label: 'Name',
  where: 'x',
  items: ['a', 'b', 'c', 'd', 'e', 'x'].map((id) => ({ id, name: id, build: () => null })),
  defaults: [item('a'), item('b'), SEP, group('g', 'More', [item('c'), item('d')]), item('e')],
};

/** `a b - g[c d] e` */
function show(nodes: Node[]): string {
  return nodes
    .map((n) => (n.t === 'sep' ? '-' : n.t === 'group' ? `${n.id}[${show(n.children)}]` : n.t === 'item' && n.hidden ? `(${n.id})` : n.id))
    .join(' ');
}

test('the fixture reads as expected', () => {
  assert.equal(show(def.defaults), 'a b - g[c d] e');
});

test('move reorders at the top and between top and group', () => {
  assert.equal(show(move(def.defaults, [0], { into: null, index: 1 })), 'b a - g[c d] e');
  assert.equal(show(move(def.defaults, [4], { into: 'g', index: 1 })), 'a b - g[c e d]');
  assert.equal(show(move(def.defaults, [3, 0], { into: null, index: 0 })), 'c a b - g[d] e');
});

test('ops leave their input alone', () => {
  const before = JSON.stringify(def.defaults);
  move(def.defaults, [0], { into: 'g', index: 0 });
  rename(def.defaults, [0], 'Z');
  setHidden(def.defaults, [0], true);
  remove(def.defaults, [3]);
  addGroup(def.defaults, 'New');
  assert.equal(JSON.stringify(def.defaults), before);
});

test('moveInto appends, also within the same group', () => {
  assert.equal(show(moveInto(def.defaults, [0], 'g')), 'b - g[c d a] e');
  assert.equal(show(moveInto(def.defaults, [3, 0], 'g')), 'a b - g[d c] e');
});

test('groups never nest', () => {
  const two = addGroup(def.defaults, 'Two');
  const id = two.at(-1);
  assert.ok(id && id.t === 'group');
  assert.equal(show(moveInto(two, [3], id.id)), show(two));
  assert.equal(show(move(two, [3], { into: id.id, index: 0 })), show(two));
  // A new group aimed into a group lands at the top.
  assert.equal(show(addGroup(def.defaults, 'Q', { into: 'g', index: 0 })).startsWith('group.1[] a'), true);
});

test('outOf puts a row at the top right after its group', () => {
  assert.equal(show(outOf(def.defaults, [3, 1])), 'a b - g[c] d e');
  assert.equal(show(outOf(def.defaults, [0])), show(def.defaults));
});

test('nudge steps within a list and stops at the ends', () => {
  assert.equal(show(nudge(def.defaults, [1], -1)), 'b a - g[c d] e');
  assert.equal(show(nudge(def.defaults, [0], -1)), show(def.defaults));
  assert.equal(show(nudge(def.defaults, [3, 0], 1)), 'a b - g[d c] e');
  assert.equal(show(nudge(def.defaults, [3, 1], 1)), show(def.defaults));
});

test('indent moves a row into the group above', () => {
  assert.equal(show(indent(def.defaults, [4])), 'a b - g[c d e]');
  assert.equal(show(indent(def.defaults, [1])), show(def.defaults));
  assert.equal(show(indent(def.defaults, [0])), show(def.defaults));
});

test('rename and hide', () => {
  const r = rename(def.defaults, [0], '  Mine  ');
  assert.deepEqual(nodeAt(r, [0]), { t: 'item', id: 'a', label: 'Mine', hidden: false });
  assert.equal((nodeAt(rename(r, [0], ' '), [0]) as { label: string | null }).label, null, 'empty goes back to the built-in name');
  assert.equal((nodeAt(rename(def.defaults, [3], ''), [3]) as { label: string }).label, 'More', 'a group keeps its name');
  assert.equal(((nodeAt(rename(def.defaults, [0], 'x'.repeat(80)), [0]) as { label: string }).label).length, 40);
  assert.equal(show(setHidden(def.defaults, [3, 1], true)), 'a b - g[c (d)] e');
  assert.equal(show(rename(def.defaults, [2], 'x')), show(def.defaults), 'lines have no name');
});

test('add: items once, groups and lines and custom entries with fresh ids', () => {
  assert.equal(show(addItem(def.defaults, 'a')), show(def.defaults));
  assert.equal(show(addItem(def.defaults, 'x', { into: 'g', index: 0 })), 'a b - g[x c d] e');
  const g1 = addGroup(def.defaults, 'Look up');
  const g2 = addGroup(g1, 'Look up');
  assert.deepEqual(
    g2.filter((n) => n.t === 'group').map((n) => (n as { id: string }).id),
    ['g', 'group.1', 'group.2'],
  );
  assert.equal(show(addSep(def.defaults, { into: null, index: 0 })), '- a b - g[c d] e');
  const action = { kind: 'link', url: 'https://x.tv/$(nick)' } as const;
  const c = addCustom(def.defaults, { label: ' Open ', action });
  assert.deepEqual(c.at(-1), { t: 'custom', id: 'custom.1', label: 'Open', icon: null, hidden: false, action });
  assert.equal(addCustom(def.defaults, { label: ' ', action }).length, def.defaults.length);
  assert.equal(addCustom(c, { label: 'Two', action }).at(-1)?.t === 'custom' && (addCustom(c, { label: 'Two', action }).at(-1) as { id: string }).id, 'custom.2');
});

test('remove: groups give their rows back to the top, items can only be hidden', () => {
  assert.equal(show(remove(def.defaults, [3])), 'a b - c d e');
  assert.equal(show(remove(def.defaults, [2])), 'a b g[c d] e');
  assert.equal(show(remove(def.defaults, [0])), show(def.defaults));
  const c = addCustom(def.defaults, { label: 'Open', action: { kind: 'copy', text: 'x' } });
  assert.equal(show(remove(c, [5])), show(def.defaults));
});

test('pathOf and nodeAt', () => {
  assert.deepEqual(pathOf(def.defaults, 'd'), [3, 1]);
  assert.deepEqual(pathOf(def.defaults, 'g'), [3]);
  assert.equal(pathOf(def.defaults, 'nope'), null);
  assert.equal(nodeAt(def.defaults, [9]), null);
  assert.equal(nodeAt(def.defaults, [0, 0, 0]), null);
});

test('reset, defaults detection and the patches to save', () => {
  const edited = move(def.defaults, [0], { into: null, index: 1 });
  assert.equal(isDefault(def, def.defaults), true);
  assert.equal(isDefault(def, edited), false);
  assert.deepEqual(savePatch(def, def.defaults), { layouts: { 'chat.name': null } });
  const patch = savePatch(def, edited);
  assert.deepEqual(patch.layouts['chat.name']?.known, ['a', 'b', 'c', 'd', 'e', 'x']);
  assert.equal(show(patch.layouts['chat.name']?.nodes ?? []), 'b a - g[c d] e');
  assert.deepEqual(resetPatch('tabs.tab'), { layouts: { 'tabs.tab': null } });
  assert.equal(show(reset(def)), 'a b - g[c d] e');
});

test('the palette lists registry items the tree has no row for', () => {
  assert.deepEqual(
    unused(def, def.defaults).map((i) => i.id),
    ['x'],
  );
  assert.deepEqual(unused(def, addItem(def.defaults, 'x')), []);
});
