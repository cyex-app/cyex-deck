import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ExtrasPreview } from '../../../ipc/bindings/ExtrasPreview';
import { doneText, extrasHeadline, highlightCanImport, highlightHeadline, usersUndo } from './importAll.ts';

const extras = (p: Partial<ExtrasPreview>): ExtrasPreview => ({
  ignores: 0,
  never: 0,
  muted: 0,
  filters: 0,
  nicknames: 0,
  already: 0,
  skipped: [],
  ...p,
});

test('V111-60 extras headline names each kind', () => {
  assert.equal(extrasHeadline(extras({})), 'Nothing new to import.');
  assert.equal(extrasHeadline(extras({ ignores: 2, nicknames: 1 })), '3 new: 2 ignores, 1 nickname.');
  assert.equal(extrasHeadline(extras({ never: 1200 })), '1.200 new: 1.200 never highlight.');
});

test('V111-60 highlight texts moved from the dialog stay the same', () => {
  const p = { phrases: 2, users: 1, badges: 0, duplicates: [], invalid: [] };
  assert.equal(highlightHeadline(p), '3 new: 2 words, 1 person.');
  assert.equal(highlightCanImport({ ...p, phrases: 0, users: 0, duplicates: ['x'] }, 'mine'), false);
  assert.equal(highlightCanImport({ ...p, phrases: 0, users: 0, duplicates: ['x'] }, 'theirs'), true);
});

test('V111-60 one toast for the whole import', () => {
  const none = { tabs: 0, highlights: 0, commands: 0, extras: 0, failed: [] };
  assert.equal(doneText(none), 'Nothing new.');
  assert.equal(doneText({ ...none, tabs: 1 }), 'Imported 1 tab.');
  assert.equal(doneText({ ...none, tabs: 2, highlights: 5, commands: 3 }), 'Imported 2 tabs, 5 highlights and 3 commands.');
  assert.equal(doneText({ ...none, extras: 4, failed: ['Tabs: no account.'] }), 'Imported 4 more. Tabs: no account.');
});

test('V111-60 undo drops new people notes and puts old ones back', () => {
  const before = { a: { nickname: 'A', note: '', color: '' } };
  const now = { a: { nickname: 'A', note: '', color: '' }, b: { nickname: 'B', note: '', color: '' } };
  assert.deepEqual(usersUndo(before, now), { a: before.a, b: null });
});
