import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { CommandItem } from '../../../ipc/bindings/CommandItem';
import type { CommandPreview } from '../../../ipc/bindings/CommandPreview';
import { headline, limitText, outcome, resultText } from './commandImport.ts';

const item = (id: number, name: string, conflict: CommandItem['conflict'] = 'none', menu: string | null = null): CommandItem => ({
  id,
  name,
  row: `${name} x`,
  notes: [],
  conflict,
  yours: conflict === 'yours' ? `${name} mine` : null,
  menu,
  menuNote: null,
});

const preview = (items: CommandItem[], room = 500): CommandPreview => ({ items, skipped: [], room, menuRoom: 500, limit: 500, unread: 0 });

test('conflicts keep yours unless taken', () => {
  const p = preview([item(0, '/a'), item(1, '/b', 'yours', '.b=x'), item(2, '/ban', 'builtIn'), item(3, '/c', 'none', '.c=y')]);
  assert.deepEqual(outcome(p, new Set()), { added: 2, replaced: 0, kept: 2, cut: [], menu: 1 });
  assert.deepEqual(outcome(p, new Set([1, 2])), { added: 3, replaced: 1, kept: 0, cut: [], menu: 2 });
});

test('the limit cuts the last new ones; a replace needs no room', () => {
  const p = preview([item(0, '/a'), item(1, '/b', 'yours'), item(2, '/c'), item(3, '/d', 'none', '.d=x')], 1);
  const o = outcome(p, new Set([1]));
  assert.deepEqual(o, { added: 1, replaced: 1, kept: 0, cut: ['/c', '/d'], menu: 0 });
  assert.equal(limitText(p, o), 'Limit is 500. 2 commands won’t fit: /c, /d.');
  assert.equal(limitText(p, outcome(preview([item(0, '/a')]), new Set())), null);
  const many = preview(Array.from({ length: 1200 }, (_, i) => item(i, `/c${i}`)), 0);
  assert.equal(limitText(many, outcome(many, new Set()), 2), 'Limit is 500. 1.200 commands won’t fit: /c0, /c1 and 1.198 more.');
});

test('texts', () => {
  assert.equal(headline(preview([])), 'Nothing to import.');
  assert.equal(headline(preview([item(0, '/a')])), '1 command.');
  assert.equal(headline(preview([item(0, '/a'), item(1, '/b', 'yours')])), '2 commands, 1 with a name in use.');
  const r = { added: 1200, replaced: 2, kept: 0, cut: ['/x'], menu: 3, menuCut: 0 };
  assert.equal(resultText(r), 'Imported 1.200, replaced 2, 3 in the message menu, 1 over the limit.');
  assert.equal(resultText({ ...r, added: 0, menu: 0, cut: [] }), 'Replaced 2.');
  assert.equal(resultText({ added: 0, replaced: 0, kept: 4, cut: [], menu: 0, menuCut: 0 }), 'Nothing new.');
});
