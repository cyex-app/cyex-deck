import assert from 'node:assert/strict';
import { test } from 'node:test';

import { countLabel, emptyQuery, filterItems, rememberSort, type Chip, type Query } from './filter.ts';

const NEW = { key: 'time', desc: true } as const;
const AZ = { key: 'name', desc: false } as const;

const it = (name: string, o: Partial<{ animated: boolean; added: number }> = {}) => ({
  name,
  animated: false,
  added: 0,
  ...o,
});
const q = (o: Partial<Omit<Query, 'chips'>> & { chips?: Chip[] } = {}): Query => ({
  ...emptyQuery(),
  ...o,
  chips: new Set(o.chips ?? []),
});
const names = (r: { name: string }[]) => r.map((x) => x.name);

const items = [
  it('Alpha', { animated: true, added: 3 }),
  it('Beta', { added: 2 }),
  it('Gamma', { animated: true, added: 9 }),
];

test('chips', () => {
  assert.deepEqual(names(filterItems(items, q({ chips: ['animated'] }))), ['Gamma', 'Alpha']);
  assert.deepEqual(names(filterItems(items, q({ chips: ['static'] }))), ['Beta']);
  assert.equal(filterItems(items, q({ chips: ['animated', 'static'] })).length, 3);
});

test('search ranks before sort', () => {
  const list = [it('Comfort', { added: 9 }), it('Forest Spring', { added: 1 }), it('Nope')];
  assert.deepEqual(names(filterItems(list, q({ text: 'for' }))), ['Forest Spring', 'Comfort']);
});

test('sort and ties', () => {
  const list = [it('b', { added: 1 }), it('A', { added: 5 }), it('c', { added: 5 })];
  assert.deepEqual(names(filterItems(list, q({ sort: AZ }))), ['A', 'b', 'c']);
  assert.equal(names(filterItems(list, q({ sort: NEW })))[2], 'b');
  const ties = [it('Fox2', { added: 1 }), it('Fox1', { added: 2 })];
  assert.deepEqual(names(filterItems(ties, q({ text: 'fox', sort: NEW }))), ['Fox1', 'Fox2']);
  assert.deepEqual(names(filterItems(ties, q({ text: 'fox', sort: AZ }))), ['Fox1', 'Fox2']);
});

test('both keys, both ways, ties by name', () => {
  const list = [it('b10', { added: 5 }), it('B2', { added: 5 }), it('a', { added: 1 }), it('c', { added: 9 })];
  const run = (key: Query['sort']['key'], desc: boolean) => names(filterItems(list, q({ sort: { key, desc } })));
  assert.deepEqual(run('time', true), ['c', 'B2', 'b10', 'a']);
  assert.deepEqual(run('time', false), ['a', 'B2', 'b10', 'c']);
  assert.deepEqual(run('name', false), ['a', 'B2', 'b10', 'c']);
  assert.deepEqual(run('name', true), ['c', 'b10', 'B2', 'a']);
});

test('the sort is remembered per pane', () => {
  assert.deepEqual(emptyQuery('badge').sort, NEW);
  rememberSort('paint', q({ sort: { key: 'name', desc: true } }));
  assert.deepEqual(emptyQuery('paint').sort, { key: 'name', desc: true });
  assert.deepEqual(emptyQuery('badge').sort, NEW);
  rememberSort('paint', q());
});

test('2300 items filter fast', () => {
  const big = Array.from({ length: 2300 }, (_, i) => it(`Paint ${i} ${i % 7 ? 'sun' : 'moon'}`, { added: i }));
  const t = performance.now();
  filterItems(big, q({ text: 'sun' }));
  assert.ok(performance.now() - t < 100, 'loose guard around 20 ms');
});

test('countLabel: matches show against the total (V1051-03)', () => {
  assert.equal(countLabel(2000, 2000), '2.000');
  assert.equal(countLabel(3, 2000), '3 of 2.000');
  assert.equal(countLabel(0, 2000), '0 of 2.000');
});

test('rows with no date go last both ways', () => {
  const list = [it('none'), it('old', { added: 1 }), it('new', { added: 9 })];
  const run = (desc: boolean) => names(filterItems(list, q({ sort: { key: 'time', desc } })));
  assert.deepEqual(run(true), ['new', 'old', 'none']);
  assert.deepEqual(run(false), ['old', 'new', 'none']);
});
