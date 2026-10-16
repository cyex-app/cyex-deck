// V1051-04 docs/commands/registry.json: the settings, hotkeys and features sections against
// this registry, hotkeys.svelte.ts and docs/commands/features.json. Commands, variables and
// functions are checked in Rust (app/src/chat/run/registry/tests.rs).

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

// @ts-expect-error plain JS module outside ui/
import { build, REGISTRY } from '../../../../../tools/commands/sections.mjs';

const STALE = 'docs/commands/registry.json is stale: run tools/commands/registry.ps1';
const file = JSON.parse(readFileSync(REGISTRY, 'utf8'));
const now = await build();

test('V1051-04 registry.json settings match the settings registry', () => {
  assert.deepEqual(file.settings, now.settings, STALE);
  assert.equal(file.changesSince, now.changesSince, STALE);
});

test('V1051-04 registry.json hotkeys match the hotkey list', () => {
  assert.deepEqual(file.hotkeys, now.hotkeys, STALE);
  const ids = now.hotkeys.map((h: { id: string }) => h.id);
  assert.equal(new Set(ids).size, ids.length, 'unique hotkey ids');
});

test('V1051-04 registry.json features match features.json', () => {
  assert.deepEqual(file.features, now.features, STALE);
});

test('V1051-04 features read as site text', () => {
  assert.match(now.changesSince, /^\d{4}\/\d{2}\/\d{2}$/, 'date as yyyy/mm/dd');
  for (const f of now.features as { id: string; title: string; summary: string; before: string }[]) {
    for (const text of [f.title, f.summary, f.before]) {
      assert.ok(!text.includes("'"), `${f.id}: curly ’, not '`);
      assert.ok(!/\d{4}-\d{2}-\d{2}/.test(text), `${f.id}: dates as yyyy/mm/dd`);
    }
    const sentences = f.summary.split(/(?<=[.!?”])\s+(?=[A-Z“])/).length;
    assert.ok(sentences <= 3, `${f.id}: 1 to 3 sentences, has ${sentences}`);
  }
});
