import assert from 'node:assert/strict';
import { test } from 'node:test';

import { group, item, type MenuDef, SEP } from '../../../components/menus/types.ts';
import { effectiveNodes } from '../../../components/menus/resolve.ts';
import { addList, childRows, isChanged, MOD_HINT, moveNote, rootRows, type SampleOf, treeOrder } from './model.ts';

type Ctx = { mod: boolean };

const def: MenuDef<Ctx> = {
  id: 'chat.name',
  label: 'Name',
  where: 'x',
  items: [
    { id: 'copy', name: 'Copy name', icon: 'copy', build: () => ({ label: 'Copy name', icon: 'copy', run() {} }) },
    { id: 'ban', name: 'Ban', icon: 'ban', when: 'mods', mod: true, build: (c) => (c.mod ? { label: 'Ban', icon: 'ban', danger: true, run() {} } : null) },
    { id: 'time', name: 'Time out', when: 'mods', mod: true, build: (c) => (c.mod ? { label: 'Time out', items: [] } : null) },
    { id: 'mine', name: 'Your entries', slot: true, build: () => null },
    { id: 'fix', name: 'Pause', renamable: false, build: () => ({ label: 'Pause', hint: 'K' }) },
  ],
  defaults: [item('copy'), group('g', 'Moderate', [item('ban'), item('time')]), SEP, item('fix'), item('mine')],
};
const sample: SampleOf = (_id, who) => ({ mod: who === 'mod' });

test('rows carry the built-in name, icon, hint and the dim tag', () => {
  const rows = rootRows(def, def.defaults, 'mod', sample);
  assert.deepEqual(rows.map((r) => r.kind), ['item', 'group', 'sep', 'item', 'item']);
  assert.equal(rows[0].label, 'Copy name');
  assert.equal(rows[0].icon, 'copy');
  assert.equal(rows[1].count, 2);
  assert.equal(rows[3].hint, 'K');
  assert.equal(rows[3].renamable, false);
  assert.equal(rows[4].slot, true);
});

test('V110-B8 mod rows and a group of them are drawn for a Mod preview only', () => {
  assert.deepEqual(childRows(def, def.defaults, 'g', 'viewer', sample), []);
  assert.deepEqual(rootRows(def, def.defaults, 'viewer', sample).map((r) => r.id), ['copy', '', 'fix', 'mine']);
  assert.deepEqual(rootRows(def, def.defaults, 'out', sample).map((r) => r.id), ['copy', '', 'fix', 'mine']);
  assert.equal(rootRows(def, def.defaults, 'mod', sample)[1].modOnly, true);
  const mod = childRows(def, def.defaults, 'g', 'mod', sample);
  assert.deepEqual(mod.map((r) => [r.id, r.absent, r.danger, r.sub, r.tag]), [['ban', false, true, false, 'mods'], ['time', false, false, true, 'mods']]);
  // Without a sample nothing is marked absent.
  assert.equal(childRows(def, def.defaults, 'g', 'mod')[0].absent, false);
});

test('V110-B8 a viewer row still shows as absent when only its condition fails', () => {
  const one: MenuDef<Ctx> = { ...def, items: [...def.items, { id: 'solo', name: 'Solo', when: 'signed in', build: () => null }], defaults: [item('solo')] };
  assert.deepEqual(rootRows(one, one.defaults, 'viewer', sample).map((r) => [r.id, r.absent, r.modOnly]), [['solo', true, false]]);
});

test('V110-B8 the Add item list keeps mod rows in a Moderator group with its hint', () => {
  const list = addList(def.items, () => {});
  const labels = list.map((e) => (e === 'separator' ? '-' : e.label));
  assert.deepEqual(labels, ['Copy name', 'Your entries', 'Pause', '-', 'Moderator']);
  const mod = list.at(-1);
  assert.ok(mod !== 'separator' && mod?.items);
  assert.equal(mod.note, MOD_HINT);
  assert.deepEqual(mod.items!.map((e) => (e === 'separator' ? '-' : e.label)), [MOD_HINT, '-', 'Ban', 'Time out']);
  assert.equal(MOD_HINT, 'Shown in chats you moderate.');
  assert.equal((mod.items![0] as { disabled?: boolean }).disabled, true);
  // A menu with no mod rows has no Moderator entry.
  assert.deepEqual(addList(def.items.filter((i) => !i.mod), () => {}).map((e) => (e === 'separator' ? '-' : e.label)), ['Copy name', 'Your entries', 'Pause']);
});

test('own names show, except on slots and fixed labels', () => {
  const nodes = structuredClone(def.defaults);
  (nodes[0] as { label: string | null }).label = 'Copy it';
  (nodes[3] as { label: string | null }).label = 'Nope';
  (nodes[4] as { label: string | null }).label = 'Nope';
  const rows = rootRows(def, nodes, 'mod', sample);
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

test('a layout read from the settings store (a proxy) is read, not refused', () => {
  const stored = new Proxy({ nodes: structuredClone(def.defaults), known: def.items.map((i) => i.id) }, {});
  assert.equal(effectiveNodes(def, stored).length, def.defaults.length);
  assert.equal(isChanged(def, stored), false);
});
