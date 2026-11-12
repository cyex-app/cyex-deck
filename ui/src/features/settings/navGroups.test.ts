// V110-N6
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { groupShown, readFolded, saveFolded, toggleFolded, unfoldFor } from './navGroups.ts';
import { PAGES } from './registry/pages.ts';

test("a folded group hides, even the open page's, unless a search or a narrow list shows all", () => {
  assert.equal(groupShown('Chat', ['Chat'], false), false);
  assert.equal(groupShown('Chat', ['Chat'], true), true);
  assert.equal(groupShown('Chat', [], false), true);
});

test('going to a page unfolds its group and leaves the others folded', () => {
  assert.deepEqual(unfoldFor(['Chat', 'App'], 'Chat'), ['App']);
  assert.deepEqual(unfoldFor(['App'], 'Chat'), ['App']);
});

test('every sidebar page has a hint the section head reads, and the nav tooltip shows it', () => {
  const nav = readFileSync(new URL('./SettingsView.svelte', import.meta.url), 'utf8');
  assert.match(nav, /hint: p\.hint/);
  for (const p of PAGES) assert.ok(p.hint.length > 0, p.id);
});

test('toggling adds and removes a name', () => {
  assert.deepEqual(toggleFolded([], 'Chat'), ['Chat']);
  assert.deepEqual(toggleFolded(['Chat', 'App'], 'Chat'), ['App']);
});

test('storage that throws or holds junk counts as none folded', () => {
  const g = globalThis as { localStorage?: unknown };
  const old = g.localStorage;
  g.localStorage = {
    getItem: () => {
      throw new Error('blocked');
    },
    setItem: () => {
      throw new Error('blocked');
    },
  };
  assert.deepEqual(readFolded(), []);
  assert.doesNotThrow(() => saveFolded(['Chat']));
  g.localStorage = { getItem: () => '{"a":1}', setItem: () => {} };
  assert.deepEqual(readFolded(), []);
  g.localStorage = { getItem: () => '["Chat",3]', setItem: () => {} };
  assert.deepEqual(readFolded(), ['Chat']);
  g.localStorage = old;
});

test('the sidebar labels are buttons with aria-expanded and fold with reduced motion respected', () => {
  const src = readFileSync(new URL('./SettingsView.svelte', import.meta.url), 'utf8');
  assert.match(src, /aria-expanded=\{shown\}/);
  assert.match(src, /groupShown\(/);
  assert.match(src, /prefers-reduced-motion/);
});
