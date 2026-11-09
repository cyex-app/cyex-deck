import assert from 'node:assert/strict';
import { test } from 'node:test';

import { group, item, type MenuDef, SEP } from '../../../components/menus/types.ts';
import { childRows, isChanged, moveNote, rootRows, type SampleOf, treeOrder } from './model.ts';

type Ctx = { mod: boolean };

const def: MenuDef<Ctx> = {
  id: 'chat.name',
  label: 'Name',
  where: 'x',
  items: [
    { id: 'copy', name: 'Copy name', icon: 'copy', build: () => ({ label: 'Copy name', icon: 'copy', run() {} }) },
    { id: 'ban', name: 'Ban', icon: 'ban', when: 'mods', build: (c) => (c.mod ? { label: 'Ban', icon: 'ban', danger: true, run() {} } : null) },
    { id: 'time', name: 'Time out', when: 'mods', build: (c) => (c.mod ? { label: 'Time out', items: [] } : null) },
    { id: 'mine', name: 'Your entries', slot: true, build: () => null },
    { id: 'fix', name: 'Pause', renamable: false, build: () => ({ label: 'Pause', hint: 'K' }) },
  ],
  defaults: [item('copy'), group('g', 'Moderate', [item('ban'), item('time')]), SEP, item('fix'), item('mine')],
};
const sample: SampleOf = (_id, who) => ({ mod: who === 'mod' });

test('rows carry the built-in name, icon, hint and the dim tag', () => {
  const rows = rootRows(def, def.defaults, 'viewer', sample);
  assert.deepEqual(rows.map((r) => r.kind), ['item', 'group', 'sep', 'item', 'item']);
  assert.equal(rows[0].label, 'Copy name');
  assert.equal(rows[0].icon, 'copy');
  assert.equal(rows[1].count, 2);
  assert.equal(rows[3].hint, 'K');
  assert.equal(rows[3].renamable, false);
  assert.equal(rows[4].slot, true);
});

test('a row whose condition fails is absent for that viewer, present for a mod', () => {
  const viewer = childRows(def, def.defaults, 'g', 'viewer', sample);
  assert.deepEqual(viewer.map((r) => [r.id, r.absent, r.tag]), [['ban', true, 'mods'], ['time', true, 'mods']]);
  const mod = childRows(def, def.defaults, 'g', 'mod', sample);
  assert.deepEqual(mod.map((r) => [r.id, r.absent, r.danger, r.sub]), [['ban', false, true, false], ['time', false, false, true]]);
  // Without a sample nothing is marked absent, the tag still shows.
  assert.equal(childRows(def, def.defaults, 'g', 'viewer')[0].absent, false);
});

test('own names show, except on slots and fixed labels', () => {
  const nodes = structuredClone(def.defaults);
  (nodes[0] as { label: string | null }).label = 'Copy it';
  (nodes[3] as { label: string | null }).label = 'Nope';
  (nodes[4] as { label: string | null }).label = 'Nope';
  const rows = rootRows(def, nodes, 'viewer', sample);
  assert.equal(rows[0].label, 'Copy it');
  assert.equal(rows[0].renamed, true);
  assert.equal(rows[0].builtIn, 'Copy name');
  assert.equal(rows[3].label, 'Pause');
  assert.equal(rows[4].label, 'Your entries');
});

test('tree order follows a group with its open rows', () => {
  assert.deepEqual(treeOrder(def, def.defaults, 'g', 'mod', sample).map((r) => r.id), ['copy', 'g', 'ban', 'time', '', 'fix', 'mine']);
  assert.deepEqual(treeOrder(def, def.defaults, null, 'mod', sample).map((r) => r.id), ['copy', 'g', '', 'fix', 'mine']);
});

test('Changed only when the arrangement differs from the built-in one', () => {
  assert.equal(isChanged(def, undefined), false);
  assert.equal(isChanged(def, { nodes: structuredClone(def.defaults), known: def.items.map((i) => i.id) }), false);
  const nodes = structuredClone(def.defaults);
  nodes.reverse();
  assert.equal(isChanged(def, { nodes, known: def.items.map((i) => i.id) }), true);
});

test('moveNote says where a row went', () => {
  const before = structuredClone(def.defaults);
  const into = structuredClone(before);
  const [c] = into.splice(0, 1);
  (into[0] as { children: unknown[] }).children.push(c);
  assert.equal(moveNote(before, into, 'copy', 'Copy name'), 'Moved Copy name into Moderate');
  assert.equal(moveNote(into, before, 'copy', 'Copy name'), 'Moved Copy name out of Moderate');
  const swapped = structuredClone(before);
  swapped.reverse();
  assert.equal(moveNote(before, swapped, 'fix', 'Pause'), 'Moved Pause to 2 of 5');
});
