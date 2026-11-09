// V110-F88 A repeating timer must pause while the window is hidden (idle CPU target): use
// everyShown (lib/awake.ts) or nowSecond. A raw setInterval needs a line here, with its reason.
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { test } from 'node:test';

const SRC = new URL('../', import.meta.url);

const ALLOWED: Record<string, string> = {
  'lib/awake.ts': 'the helper itself',
  'features/player/engine.svelte.ts': 'stall check and paused-behind clock: must keep running while the window is hidden so a stalled live stream still recovers',
  'features/player/rewind.svelte.ts': 'keeps its own clock and catches up on show through onShown; the position runs while hidden',
  'features/chat/ChatInput.svelte': 'chat box owned by another change in this run (F4): move to everyShown there',
};

test('no raw setInterval outside the allow-list', () => {
  const files = readdirSync(SRC, { recursive: true, encoding: 'utf8' })
    .map((f) => f.split(String.fromCharCode(92)).join('/'))
    .filter((f) => /\.(svelte|ts)$/.test(f) && !/\.test\.ts$/.test(f));
  const bad = files.filter((f) => !(f in ALLOWED) && /\bsetInterval\(/.test(readFileSync(new URL(f, SRC), 'utf8')));
  assert.deepEqual(bad, [], 'use everyShown from lib/awake.ts, or allow-list the file with a reason');
});

test('every allow-list entry still has a setInterval', () => {
  for (const f of Object.keys(ALLOWED)) {
    assert.match(readFileSync(new URL(f, SRC), 'utf8'), /\bsetInterval\(/, `${f} no longer needs its entry`);
  }
});
