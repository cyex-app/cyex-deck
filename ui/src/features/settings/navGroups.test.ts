// V110-N6
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { groupShown, readFolded, saveFolded, toggleFolded } from './navGroups.ts';

test('a folded group hides unless it holds the open page, a search or a narrow list', () => {
  assert.equal(groupShown('Chat', ['Chat'], 'App', false), false);
  assert.equal(groupShown('Chat', ['Chat'], 'Chat', false), true);
  assert.equal(groupShown('Chat', ['Chat'], 'App', true), true);
  assert.equal(groupShown('Chat', [], 'App', false), true);
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
