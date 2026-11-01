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
    // Lets the queue start the next ones.
    for (let i = 0; i < 5; i++) await Promise.resolve();
  };
  return { calls, ask, answer };
}

const tick = async () => {
  for (let i = 0; i < 5; i++) await Promise.resolve();
};

test('one link is asked once, a hover joins the prefetch (V108-C25)', async () => {
  const a = asker();
  const q = linkQueue(a.ask);
  q.prefetch('x');
  q.prefetch('x');
  await tick();
  const hover = q.get('x');
  await tick();
  assert.deepEqual(a.calls, ['x']);
  await a.answer('x');
  assert.equal(await hover, 'card x');
  // Asked lately: a prefetch leaves it to the Rust cache.
  q.prefetch('x');
  await tick();
  assert.deepEqual(a.calls, ['x']);
});

test('at most `limit` at once, newest first', async () => {
  const a = asker();
  const q = linkQueue(a.ask, { limit: 2 });
  for (const u of ['a', 'b', 'c', 'd']) q.prefetch(u);
  await tick();
  assert.equal(a.calls.length, 2);
  assert.deepEqual(q.stats(), { running: 2, waiting: 2 });
  await a.answer(a.calls[0]);
  assert.equal(a.calls.length, 3);
  assert.equal(a.calls[2], 'd');
});

test('a hover goes ahead of the queue', async () => {
  const a = asker();
  const q = linkQueue(a.ask, { limit: 1 });
  q.prefetch('a');
  q.prefetch('b');
  await tick();
  void q.get('b');
  await tick();
  assert.deepEqual(a.calls, ['a', 'b']);
  assert.equal(q.stats().waiting, 0);
});

test('a long backlog drops its oldest asks', async () => {
  const a = asker();
  const q = linkQueue(a.ask, { limit: 1, backlog: 2 });
  for (const u of ['a', 'b', 'c', 'd', 'e']) q.prefetch(u);
  await tick();
  assert.deepEqual(q.stats(), { running: 1, waiting: 2 });
  await a.answer('a');
  await a.answer('e');
  assert.deepEqual(a.calls, ['a', 'e', 'd']);
});

test('a failed ask frees its place', async () => {
  let n = 0;
  const q = linkQueue(() => (n++ ? Promise.resolve('ok') : Promise.reject(new Error('down'))), { limit: 1 });
  await assert.rejects(q.get('x'));
  assert.equal(await q.get('x'), 'ok');
  assert.equal(q.stats().running, 0);
});
