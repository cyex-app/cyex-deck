import assert from 'node:assert/strict';
import { test } from 'node:test';

import { linkQueue } from './linkQueue.ts';

/** Asks that answer when told to. */
function asker() {
  const calls: string[] = [];
  const open = new Map<string, (v: string) => void>();
  const ask = (url: string) => {
    calls.push(url);
    return new Promise<string>((resolve) => open.set(url, resolve));
  };
  const answer = async (url: string) => {
    open.get(url)!(`card ${url}`);
    open.delete(url);
    for (let i = 0; i < 5; i++) await Promise.resolve();
  };
  return { calls, ask, answer };
}

const tick = async () => {
  for (let i = 0; i < 5; i++) await Promise.resolve();
};

test('a second hover joins the ask already out (V108-C25)', async () => {
  const a = asker();
  const q = linkQueue(a.ask, Infinity, Infinity);
  const first = q.get('x');
  const second = q.get('x');
  await tick();
  assert.deepEqual(a.calls, ['x']);
  assert.deepEqual(q.stats(), { running: 1 });
  await a.answer('x');
  assert.equal(await first, 'card x');
  assert.equal(await second, 'card x');
  assert.deepEqual(q.stats(), { running: 0 });
});

test('different links are asked side by side', async () => {
  const a = asker();
  const q = linkQueue(a.ask, Infinity, Infinity);
  void q.get('a');
  void q.get('b');
  await tick();
  assert.deepEqual(a.calls, ['a', 'b']);
});

test('a failed ask frees its place', async () => {
  let n = 0;
  const q = linkQueue(() => (n++ ? Promise.resolve('ok') : Promise.reject(new Error('down'))));
  await assert.rejects(q.get('x'));
  assert.equal(await q.get('x'), 'ok');
  assert.equal(q.stats().running, 0);
});

test('a limit keeps only that many asks running; the rest wait in order (V110-C66)', async () => {
  const a = asker();
  const q = linkQueue(a.ask, 2, Infinity);
  const got = ['a', 'b', 'c', 'd'].map((u) => q.get(u));
  await tick();
  assert.deepEqual(a.calls, ['a', 'b']);
  await a.answer('a');
  assert.deepEqual(a.calls, ['a', 'b', 'c']);
  await a.answer('b');
  await a.answer('c');
  await a.answer('d');
  assert.deepEqual(await Promise.all(got), ['card a', 'card b', 'card c', 'card d']);
  assert.equal(q.stats().running, 0);
});

test('a hung ask fails after the limit and frees its place (V110-F96)', async (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const q = linkQueue(() => new Promise<string>(() => {}), 1, 10_000);
  const first = q.get('a');
  const second = q.get('b');
  const failed = assert.rejects(first, /timed out/);
  await tick();
  assert.deepEqual(q.stats(), { running: 2 });
  t.mock.timers.tick(10_000);
  await failed;
  // b took the freed place and is now the one waiting out its own limit.
  await tick();
  t.mock.timers.tick(10_000);
  await assert.rejects(second, /timed out/);
  await tick();
  assert.deepEqual(q.stats(), { running: 0 });
});
