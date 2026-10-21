import assert from 'node:assert/strict';
import { test } from 'node:test';

import { appended, deduped, emptySink, kept, type PageReply, Pager, toggled } from './paged.ts';

type Row = { id: string };
const rows = (...ids: string[]): Row[] => ids.map((id) => ({ id }));

/** A fetch whose replies the test hands out, in any order. */
function manual() {
  const calls: { cursor: string | null; done: (p: PageReply<Row>) => void; fail: (e: unknown) => void }[] = [];
  const fetch = (cursor: string | null) =>
    new Promise<PageReply<Row>>((done, fail) => {
      calls.push({ cursor, done, fail });
    });
  return { calls, fetch };
}

const ids = (list: Row[]) => list.map((r) => r.id);

test('a slow reply for an old filter is dropped (out of order)', async () => {
  const m = manual();
  const s = emptySink<Row>();
  const p = new Pager(s, { fetch: m.fetch, key: (r) => r.id });
  const old = p.reload({ clear: true });
  const fresh = p.reload({ clear: true });
  m.calls[1].done({ items: rows('b'), next: null, total: 1 });
  assert.equal(await fresh, true);
  m.calls[0].done({ items: rows('a'), next: 'x', total: 9 });
  assert.equal(await old, false);
  assert.deepEqual(ids(s.items), ['b']);
  assert.equal(s.total, 1);
  assert.equal(s.next, null);
  assert.equal(s.loading, false);
});

test('a reload drops a load more on its way, and a clear resets the total', async () => {
  const m = manual();
  const s = emptySink<Row>();
  const p = new Pager(s, { fetch: m.fetch, key: (r) => r.id });
  void p.reload();
  m.calls[0].done({ items: rows('a'), next: 'c1', total: 5 });
  await Promise.resolve();
  await Promise.resolve();
  const more = p.loadMore();
  assert.equal(s.more, true);
  void p.reload({ clear: true });
  assert.equal(s.total, null);
  assert.equal(s.more, false);
  m.calls[1].done({ items: rows('stale'), next: null });
  assert.equal(await more, false);
  assert.deepEqual(ids(s.items), []);
});

test('load more appends without repeats and never runs twice', async () => {
  const m = manual();
  const s = emptySink<Row>();
  const p = new Pager(s, { fetch: m.fetch, key: (r) => r.id });
  const first = p.reload();
  m.calls[0].done({ items: rows('a', 'b', 'a'), next: 'c1' });
  await first;
  assert.deepEqual(ids(s.items), ['a', 'b']);
  const more = p.loadMore();
  assert.equal(await p.loadMore(), false);
  assert.equal(m.calls.length, 2);
  assert.equal(m.calls[1].cursor, 'c1');
  m.calls[1].done({ items: rows('b', 'c'), next: null });
  await more;
  assert.deepEqual(ids(s.items), ['a', 'b', 'c']);
  assert.equal(await p.loadMore(), false);
});

test('a failed page keeps the list and says why', async () => {
  const m = manual();
  const s = emptySink<Row>();
  const p = new Pager(s, { fetch: m.fetch, key: (r) => r.id });
  const first = p.reload();
  m.calls[0].done({ items: rows('a'), next: 'c1' });
  await first;
  const more = p.loadMore();
  m.calls[1].fail('Twitch is down.');
  await more;
  assert.deepEqual(ids(s.items), ['a']);
  assert.ok(s.failed);
  assert.equal(s.more, false);
});

test('the selection drops rows that left, removal counts down the total', async () => {
  const m = manual();
  const s = emptySink<Row>();
  const p = new Pager(s, { fetch: m.fetch, key: (r) => r.id });
  const first = p.reload();
  m.calls[0].done({ items: rows('a', 'b', 'c'), next: null, total: 3 });
  await first;
  p.pickAll(true);
  p.toggle('a');
  assert.deepEqual([...s.picked].sort(), ['b', 'c']);
  assert.deepEqual(ids(p.pickedItems()), ['b', 'c']);
  p.remove(['b']);
  assert.deepEqual([...s.picked], ['c']);
  assert.equal(s.total, 2);
  const again = p.reload();
  m.calls[1].done({ items: rows('a'), next: null, total: 1 });
  await again;
  assert.equal(s.picked.size, 0);
  p.patch('a', () => ({ id: 'a' }));
  assert.equal(s.items.length, 1);
});

test('helpers', () => {
  const k = (r: Row) => r.id;
  assert.deepEqual(ids(deduped(rows('a', 'a', 'b'), k)), ['a', 'b']);
  assert.deepEqual(ids(appended(rows('a'), rows('a', 'b', 'b'), k)), ['a', 'b']);
  assert.deepEqual([...toggled(toggled(new Set(), 'x'), 'x')], []);
  assert.deepEqual([...kept(new Set(['x', 'y']), ['y', 'z'])], ['y']);
});
