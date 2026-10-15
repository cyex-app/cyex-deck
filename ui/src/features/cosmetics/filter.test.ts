import assert from 'node:assert/strict';
import { test } from 'node:test';

import { emptyQuery, filterItems, NEW_DAYS, type Chip, type Query } from './filter.ts';

const DAY = 86_400_000;
const NOW = Date.UTC(2026, 9, 15);
const it = (name: string, o: Partial<{ animated: boolean; usable: boolean; added: number }> = {}) => ({
  name,
  animated: false,
  usable: true,
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
  it('Beta', { usable: false, added: 2 }),
  it('Gamma', { animated: true, usable: false, added: NOW - DAY }),
];

test('chips', () => {
  assert.deepEqual(names(filterItems(items, q({ chips: ['animated'] }), NOW)), ['Gamma', 'Alpha']);
  assert.deepEqual(names(filterItems(items, q({ chips: ['static'] }), NOW)), ['Beta']);
  assert.equal(filterItems(items, q({ chips: ['animated', 'static'] }), NOW).length, 3);
  assert.deepEqual(names(filterItems(items, q({ chips: ['usable'] }), NOW)), ['Alpha']);
  assert.deepEqual(names(filterItems(items, q({ chips: ['animated', 'usable'] }), NOW)), ['Alpha']);
});

test('new is 30 days', () => {
  const edge = [it('In', { added: NOW - NEW_DAYS * DAY }), it('Out', { added: NOW - NEW_DAYS * DAY - 1 })];
  assert.deepEqual(names(filterItems(edge, q({ chips: ['new'] }), NOW)), ['In']);
});

test('search ranks before sort', () => {
  const list = [it('Comfort', { added: 9 }), it('Forest Spring', { added: 1 }), it('Nope')];
  assert.deepEqual(names(filterItems(list, q({ text: 'for' }), NOW)), ['Forest Spring', 'Comfort']);
});

test('sort and ties', () => {
  const list = [it('b', { added: 1 }), it('A', { added: 5 }), it('c', { added: 5 })];
  assert.deepEqual(names(filterItems(list, q({ sort: 'az' }), NOW)), ['A', 'b', 'c']);
  assert.equal(names(filterItems(list, q({ sort: 'newest' }), NOW))[2], 'b');
  const ties = [it('Fox2', { added: 1 }), it('Fox1', { added: 2 })];
  assert.deepEqual(names(filterItems(ties, q({ text: 'fox', sort: 'newest' }), NOW)), ['Fox1', 'Fox2']);
  assert.deepEqual(names(filterItems(ties, q({ text: 'fox', sort: 'az' }), NOW)), ['Fox1', 'Fox2']);
});

test('2300 items filter fast', () => {
  const big = Array.from({ length: 2300 }, (_, i) => it(`Paint ${i} ${i % 7 ? 'sun' : 'moon'}`, { added: i }));
  const t = performance.now();
  filterItems(big, q({ text: 'sun' }), NOW);
  assert.ok(performance.now() - t < 100, 'loose guard around 20 ms');
});
