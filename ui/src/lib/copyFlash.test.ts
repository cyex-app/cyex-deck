import assert from 'node:assert/strict';
import { test } from 'node:test';

import { COPIED_MS, copiedSaid, copyFlash } from './copyFlash.ts';

function fake() {
  const jobs = new Map<number, { run: () => void; ms: number }>();
  let n = 0;
  return {
    jobs,
    timers: {
      set: (run: () => void, ms: number) => (jobs.set(++n, { run, ms }), n),
      clear: (id: unknown) => void jobs.delete(id as number),
    },
  };
}

test('V110-A-idea the check shows, then goes out after 1,2 s', () => {
  const f = fake();
  const seen: (string | null)[] = [];
  const flash = copyFlash((k) => seen.push(k), f.timers);
  flash.show('link');
  assert.deepEqual(seen, ['link']);
  assert.equal([...f.jobs.values()][0].ms, COPIED_MS);
  [...f.jobs.values()][0].run();
  assert.deepEqual(seen, ['link', null]);
});

test('V110-A-idea a second copy moves the check and restarts the clock', () => {
  const f = fake();
  const seen: (string | null)[] = [];
  const flash = copyFlash((k) => seen.push(k), f.timers);
  flash.show('a');
  flash.show('b');
  assert.equal(f.jobs.size, 1);
  assert.deepEqual(seen, ['a', 'b']);
});

test('V110-A-idea stop clears it at once', () => {
  const f = fake();
  const seen: (string | null)[] = [];
  const flash = copyFlash((k) => seen.push(k), f.timers);
  flash.show('a');
  flash.stop();
  assert.equal(f.jobs.size, 0);
  assert.deepEqual(seen, ['a', null]);
});

test('V110-A-idea the spoken line matches the toast it replaces', () => {
  assert.equal(copiedSaid('link'), 'Link copied.');
  assert.equal(copiedSaid('Emote name'), 'Emote name copied.');
});
