import assert from 'node:assert/strict';
import { test } from 'node:test';

import { type MenuEntry, menuProblems } from '../menuItems.ts';
import { effectiveNodes, layoutOf, resolveMenu } from './resolve.ts';
import { group, item, type ItemDef, type MenuDef, SEP } from './types.ts';

interface Ctx {
  mod: boolean;
  signedIn: boolean;
}

const row = (id: string, label: string, when: (c: Ctx) => boolean = () => true): ItemDef<Ctx> => ({
  id,
  name: label,
  icon: 'user',
  build: (c) => (when(c) ? { label, icon: 'user', run: () => {} } : null),
});

export const DEF: MenuDef<Ctx> = {
  id: 'chat.name',
  label: 'Name',
  where: 'Right-click a name',
  items: [
    row('usercard', 'Usercard'),
    row('mention', 'Mention'),
    row('whisper', 'Whisper', (c) => c.signedIn),
    row('copy', 'Copy name'),
    row('timeout', 'Time out', (c) => c.mod),
    row('ban', 'Ban', (c) => c.mod),
    row('warn', 'Warn', (c) => c.mod),
    row('unban', 'Unban', (c) => c.mod),
    row('hide', 'Hide'),
    row('block', 'Block', (c) => c.signedIn),
    { id: 'slot.entries', name: 'Your entries', slot: true, build: () => [{ label: 'Mine', icon: 'user' }] },
    row('extra', 'Copy user ID'),
  ],
  defaults: [
    item('usercard'),
    item('mention'),
    item('whisper'),
    item('copy'),
    SEP,
    item('timeout'),
    item('ban'),
    SEP,
    group('moderate', 'Moderate', [item('warn'), item('unban')], 'shield'),
    group('hideblock', 'Hide or block', [item('hide'), item('block')], 'ban'),
    SEP,
    item('slot.entries'),
  ],
};

const MOD: Ctx = { mod: true, signedIn: true };
const labels = (list: MenuEntry[]): string[] => list.map((e) => (e === 'separator' ? '-' : e.items ? `${e.label}>${labels(e.items).join(',')}` : e.label));

test('no layout gives the built-in menu, the same as a layout holding the defaults', () => {
  const plain = resolveMenu(DEF, undefined, MOD);
  assert.deepEqual(labels(plain), [
    'Usercard',
    'Mention',
    'Whisper',
    'Copy name',
    '-',
    'Time out',
    'Ban',
    '-',
    'Moderate>Warn,Unban',
    'Hide or block>Hide,Block',
    '-',
    'Mine',
  ]);
  assert.deepEqual(labels(resolveMenu(DEF, layoutOf(DEF, DEF.defaults), MOD)), labels(plain));
  assert.deepEqual(effectiveNodes(DEF, undefined), DEF.defaults);
  assert.equal(menuProblems(plain).length, 0);
});

test('the defaults are not shared with the result', () => {
  const nodes = effectiveNodes(DEF, undefined);
  nodes.pop();
  assert.equal(DEF.defaults.length, 12);
});

test('conditions drop rows, and groups and lines that end up empty', () => {
  const viewer = resolveMenu(DEF, undefined, { mod: false, signedIn: false });
  assert.deepEqual(labels(viewer), ['Usercard', 'Mention', 'Copy name', '-', 'Hide or block>Hide', '-', 'Mine']);
  assert.equal(menuProblems(viewer).length, 0);
});

test('entries carry the registry id, groups their own', () => {
  const list = resolveMenu(DEF, undefined, MOD);
  assert.equal((list[0] as { id?: string }).id, 'usercard');
  assert.equal((list[8] as { id?: string }).id, 'moderate');
});

test('renames, hidden rows and moves apply', () => {
  const layout = layoutOf(DEF, [
    { t: 'item', id: 'copy', label: 'Copy login', hidden: false },
    { t: 'item', id: 'usercard', label: null, hidden: true },
    { t: 'item', id: 'mention', label: null, hidden: false },
    { t: 'item', id: 'whisper', label: null, hidden: false },
    { t: 'item', id: 'hide', label: null, hidden: false },
    { t: 'item', id: 'block', label: null, hidden: false },
    { t: 'item', id: 'timeout', label: null, hidden: false },
    { t: 'item', id: 'ban', label: null, hidden: true },
    { t: 'item', id: 'warn', label: null, hidden: false },
    { t: 'item', id: 'unban', label: null, hidden: false },
    { t: 'item', id: 'slot.entries', label: 'Renamed slot', hidden: false },
    { t: 'item', id: 'extra', label: null, hidden: false },
  ]);
  assert.deepEqual(labels(resolveMenu(DEF, layout, MOD)), [
    'Copy login',
    'Mention',
    'Whisper',
    'Hide',
    'Block',
    'Time out',
    'Warn',
    'Unban',
    'Mine',
    'Copy user ID',
  ]);
});

test('two-state labels and rows that build several rows are not renamed', () => {
  const def: MenuDef<Ctx> = {
    ...DEF,
    items: [
      { ...row('a', 'Play'), renamable: false },
      { id: 'b', name: 'Two', build: () => [{ label: 'One' }, 'separator', { label: 'Two' }] },
    ],
    defaults: [],
  };
  const layout = layoutOf(def, [
    { t: 'item', id: 'a', label: 'Start', hidden: false },
    { t: 'item', id: 'b', label: 'X', hidden: false },
  ]);
  assert.deepEqual(labels(resolveMenu(def, layout, MOD)), ['Play', 'One', '-', 'Two']);
});

test('unknown ids in a layout are dropped', () => {
  const layout = { nodes: [item('usercard'), item('gone'), group('g', 'G', [item('also-gone')])], known: DEF.items.map((i) => i.id) };
  assert.deepEqual(labels(resolveMenu(DEF, layout, MOD)), ['Usercard']);
  assert.deepEqual(
    effectiveNodes(DEF, layout).map((n) => (n.t === 'sep' ? '-' : n.id)).filter((id) => id === 'gone' || id === 'also-gone'),
    [],
  );
});

test('a new built-in item goes after the item before it in the built-in layout', () => {
  // The user's menu predates "unban" and "hide".
  const layout = {
    nodes: [item('usercard'), item('copy'), SEP, group('mine', 'Mine', [item('ban')]), item('warn'), item('slot.entries')],
    known: ['usercard', 'copy', 'ban', 'warn', 'slot.entries'],
  };
  const ids = (nodes: ReturnType<typeof effectiveNodes>) => nodes.map((n) => (n.t === 'sep' ? '-' : n.t === 'group' ? `${n.id}[${n.children.map((c) => (c.t === 'sep' ? '-' : c.id)).join(',')}]` : n.id));
  // mention: after usercard; whisper: after mention (just placed); timeout: after copy? default predecessor is "copy"
  assert.deepEqual(ids(effectiveNodes(DEF, layout)), [
    'usercard',
    'mention',
    'whisper',
    'copy',
    'timeout',
    '-',
    'mine[ban]',
    'warn',
    'unban',
    'hide',
    'block',
    'slot.entries',
  ]);
});

test('with no predecessor the item joins its built-in group, else comes before the slots', () => {
  const layout = {
    nodes: [group('moderate', 'Moderate', []), SEP, item('slot.entries')],
    known: ['slot.entries'],
  };
  const nodes = effectiveNodes(DEF, layout);
  const mod = nodes.find((n) => n.t === 'group');
  assert.ok(mod && mod.t === 'group');
  // usercard..ban find no earlier item, so they sit before the slot; warn and unban join "moderate".
  const flat = nodes.map((n) => (n.t === 'sep' ? '-' : n.id));
  assert.equal(flat.at(-1), 'slot.entries');
  assert.ok(flat.indexOf('usercard') < flat.indexOf('slot.entries'));
  assert.deepEqual(
    mod.children.map((c) => (c.t === 'sep' ? '-' : c.id)),
    ['warn', 'unban'],
  );
});

test('items outside the built-in layout are never placed on their own', () => {
  const layout = { nodes: [item('usercard')], known: [] };
  assert.ok(!idsAll(effectiveNodes(DEF, layout)).includes('extra'));
  assert.ok(idsAll(effectiveNodes(DEF, layout)).includes('copy'));
});

test('a seen item that the user took out is not put back', () => {
  const layout = { nodes: [item('usercard')], known: DEF.items.map((i) => i.id) };
  assert.deepEqual(idsAll(effectiveNodes(DEF, layout)), ['usercard']);
});

test('custom entries show through the hook only', () => {
  const layout = {
    nodes: [
      item('usercard'),
      { t: 'custom' as const, id: 'custom.1', label: 'Look up', icon: null, hidden: false, action: { kind: 'copy' as const, text: 'x' } },
    ],
    known: [],
  };
  assert.deepEqual(labels(resolveMenu(DEF, { ...layout, known: DEF.items.map((i) => i.id) }, MOD)), ['Usercard']);
  const withHook = resolveMenu(DEF, { ...layout, known: DEF.items.map((i) => i.id) }, MOD, {
    custom: (n) => ({ label: n.label, icon: 'copy' }),
  });
  assert.deepEqual(labels(withHook), ['Usercard', 'Look up']);
});

function idsAll(nodes: ReturnType<typeof effectiveNodes>): string[] {
  return nodes.flatMap((n) => (n.t === 'sep' ? [] : n.t === 'group' ? [n.id, ...idsAll(n.children)] : [n.id]));
}
