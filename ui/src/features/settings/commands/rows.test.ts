import assert from 'node:assert/strict';
import { test } from 'node:test';

import { checkLines, commandRow, commandView, hiddenBySubmenu, menuRow, menuView } from './rows.ts';

const on = (text: string) => ({ enabled: true, text });

test('commands round-trip untouched', () => {
  for (const t of ['/hi Hello $1!', '/hi  two  spaces ', '/hi#forsen hey', '_x y', '# note here', '/solo', '']) {
    assert.deepEqual(commandRow(commandView(on(t))), on(t), t);
  }
});

test('command split', () => {
  assert.deepEqual(commandView(on('/hi Hello $1!')), { enabled: true, name: '/hi', sends: 'Hello $1!', raw: '/hi Hello $1!' });
  const n = commandView(on('# note here'));
  assert.equal(n.name, '#');
  assert.equal(n.sends, 'note here');
});

test('command edits', () => {
  const v = commandView(on('/hi Hello'));
  assert.equal(commandRow({ ...v, name: '/h i' }).text, '/hi Hello');
  assert.equal(commandRow({ ...v, sends: '' }).text, '/hi');
  assert.equal(commandRow({ ...v, sends: 'Yo', enabled: false }).enabled, false);
  assert.equal(commandRow({ ...v, name: '/yo' }).text, '/yo Hello');
});

test('menus round-trip untouched', () => {
  for (const t of ['Logs=/openlogs $1', '@Copy', '.Name=/copy $1', '-', '/Shoutout', 'a=b=c', '# note x']) {
    assert.deepEqual(menuRow(menuView(on(t))), on(t), t);
  }
});

test('menu split and edits', () => {
  const v = menuView(on('a=b=c'));
  assert.equal(v.label, 'a');
  assert.equal(v.command, 'b=c');
  assert.equal(menuRow({ ...v, label: 'x=y' }).text, 'xy=b=c');
  assert.equal(menuRow({ ...v, command: '' }).text, 'a');
  assert.equal(menuView(on('@Copy')).command, '');
});

test('checkLines blanks off rows', () => {
  assert.deepEqual(checkLines([on('/a x'), { enabled: false, text: '/b y' }, on('/c')]), ['/a x', '', '/c']);
});

test('hiddenBySubmenu follows the submenu switch', () => {
  const rows = [
    { enabled: false, text: '@Copy' },
    on('.One=/1'),
    on('.Two=/2'),
    on('Top=/t'),
    on('.Orphan=/o'),
    { enabled: true, text: '@Open' },
    on('.In=/i'),
  ];
  assert.deepEqual(
    rows.map((_, i) => hiddenBySubmenu(rows, i)),
    [false, true, true, false, false, false, false],
  );
});
