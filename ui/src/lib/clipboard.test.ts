import assert from 'node:assert/strict';
import { test } from 'node:test';

// toast.svelte.ts is a .svelte.ts module: outside the compiler, $state is the plain value.
(globalThis as unknown as { $state: <T>(v: T) => T }).$state = (v) => v;
const { copy } = await import('./clipboard.ts');
const { toast } = await import('./toast.svelte.ts');

test('a copy says what was copied', async () => {
  let got = '';
  const ok = await copy('kuni', 'name', async (t) => {
    got = t;
  });
  assert.equal(ok, true);
  assert.equal(got, 'kuni');
  const last = toast.items.at(-1);
  assert.equal(last?.kind, 'success');
  assert.equal(last?.text, 'Name copied.');
});

test('a failed copy says so', async () => {
  const ok = await copy('x', 'Link', () => Promise.reject(new Error('denied')));
  assert.equal(ok, false);
  const last = toast.items.at(-1);
  assert.equal(last?.kind, 'error');
  assert.equal(last?.text, 'Couldn’t copy that. Try again.');
  for (const t of [...toast.items]) toast.dismiss(t.id);
});
