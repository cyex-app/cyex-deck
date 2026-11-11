// V110-N5 F5: R2-1, R2-2, R2-4, R2-5, R2-6.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { filtersEntry } from '../../components/menus/filtersRow.ts';
import { menuMoveTarget } from '../../components/menus/keys.ts';
import { enterMayTakeFirst } from './channelMatch.ts';
import { bootPlan } from './theatreWindow.ts';

const read = (p: string) => readFileSync(new URL(p, import.meta.url), 'utf8');

test('R2-1 a fullscreen window nobody wants is left at start', () => {
  assert.equal(bootPlan(true, false, false), 'leave');
  assert.equal(bootPlan(true, true, false), null);
  assert.equal(bootPlan(true, false, true), null);
  assert.equal(bootPlan(false, false, false), null);
  assert.match(read('../../app/App.svelte'), /windowFullscreen\.releaseStray\(\)/);
  assert.match(read('../../components/WindowControls.svelte'), /releaseStray\(\)/);
});

test('R2-2 Enter takes the first hit only for login-shaped text or after an arrow', () => {
  for (const t of ['a'.repeat(500), '<b>x</b>"q', '😀😀', 'https://www.twitch.tv/' + 'z'.repeat(40)]) assert.equal(enterMayTakeFirst(t, false), false, t.slice(0, 20));
  for (const t of ['xqc', 'https://twitch.tv/xQc', ' xQc ']) assert.equal(enterMayTakeFirst(t, false), true, t);
  assert.equal(enterMayTakeFirst('<b>x</b>', true), true);
  // A localized display name typed exactly still adds its channel; a fuzzy neighbour doesn't.
  assert.equal(enterMayTakeFirst('ﾘｱﾙ 名前', false, { login: 'realname', name: 'ﾘｱﾙ 名前' }), true);
  assert.equal(enterMayTakeFirst('aaaa aaaa', false, { login: 'cannaaaa', name: 'Cannaaaa' }), false);
  assert.match(read('./ChannelPicker.svelte'), /No channel called that\. Pick one below\./);
});

test('R2-4 entering theatre blurs a focused text field', () => {
  assert.match(read('./theatre.svelte.ts'), /enter\(tile: string\): void \{[\s\S]*?\.blur\(\)/);
});

test('R2-5 Home and End with no row focused go to the first and last row', () => {
  assert.equal(menuMoveTarget(-1, 5, 'Home'), 0);
  assert.equal(menuMoveTarget(-1, 5, 'End'), 4);
  assert.equal(menuMoveTarget(3, 5, 'Home'), 0);
  assert.equal(menuMoveTarget(-1, 5, 'ArrowDown'), 0);
  assert.equal(menuMoveTarget(-1, 5, 'ArrowUp'), 4);
  assert.equal(menuMoveTarget(4, 5, 'ArrowDown'), 4);
  assert.equal(menuMoveTarget(0, 0, 'Home'), -1);
});

test('R2-6 Filters with none made is one greyed row with the hint, not a submenu', () => {
  const e = filtersEntry([], [], () => {}) as { disabled?: boolean; note?: string; items?: unknown[] };
  assert.equal(e.disabled, true);
  assert.equal(e.note, 'Make filters in Settings → Filters');
  assert.equal(e.items, undefined);
  const w = filtersEntry([{ id: 'a', name: 'A' }], ['a'], () => {}) as { items?: { checked?: boolean }[] };
  assert.equal(w.items?.[0].checked, true);
});
