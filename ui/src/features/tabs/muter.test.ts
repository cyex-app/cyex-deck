import assert from 'node:assert/strict';
import { test } from 'node:test';

import { muter } from './muter.ts';

/** A fake Rust: each call waits until the test settles it. */
function setup() {
  let state = new Set<string>();
  const calls: { id: string; on: boolean; ok: (ids: string[]) => void; no: (e: unknown) => void }[] = [];
  const errors: unknown[] = [];
  const mute = muter({
    get: () => state,
    set: (s) => (state = s),
    send: (id, on) => new Promise((ok, no) => calls.push({ id, on, ok, no })),
    fail: (e) => errors.push(e),
  });
  return { mute, calls, errors, state: () => [...state].sort() };
}

const settle = () => new Promise((r) => setTimeout(r));

test('a mute shows at once and takes Rust’s answer', async () => {
  const s = setup();
  const done = s.mute('a', true);
  assert.deepEqual(s.state(), ['a']);
  s.calls[0].ok(['a', 'b']);
  await done;
  assert.deepEqual(s.state(), ['a', 'b']);
});

test('a failed toggle is undone', async () => {
  const s = setup();
  const done = s.mute('a', true);
  s.calls[0].no('nope');
  await done;
  assert.deepEqual(s.state(), []);
  assert.deepEqual(s.errors, ['nope']);
});

test('a failed toggle never undoes a newer one of the same tab', async () => {
  const s = setup();
  const first = s.mute('a', true);
  const second = s.mute('a', false);
  s.calls[0].no('nope');
  await first;
  assert.deepEqual(s.state(), []); // still the newer "unmute"
  s.calls[1].ok([]);
  await second;
  assert.deepEqual(s.state(), []);
});

test('an older answer doesn’t overwrite a newer toggle', async () => {
  const s = setup();
  const first = s.mute('a', true);
  const second = s.mute('b', true);
  s.calls[0].ok(['a']);
  await first;
  await settle();
  assert.deepEqual(s.state(), ['a', 'b']);
  s.calls[1].ok(['a', 'b']);
  await second;
  assert.deepEqual(s.state(), ['a', 'b']);
});
