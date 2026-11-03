import assert from 'node:assert/strict';
import { mock, test } from 'node:test';

// The store is a .svelte.ts module: outside the compiler, $state is the plain value.
(globalThis as unknown as { $state: <T>(v: T) => T }).$state = (v) => v;
const { CARD_MS, CardStore, MAX_CARDS, MIN_LEFT_MS } = await import('./noticeCards.svelte.ts');

mock.timers.enable({ apis: ['setTimeout', 'Date'] });

const card = (title: string) => ({ kind: 'drop' as const, title, text: '', image: null, open: { kind: 'drops' as const } });

test('newest on top, at most 3', () => {
  const s = new CardStore();
  for (let i = 0; i < 5; i++) s.push(card(`c${i}`));
  assert.deepEqual(s.items.map((c) => c.title), ['c4', 'c3', 'c2']);
  assert.equal(s.items.length, MAX_CARDS);
});

test('a card leaves after its time', () => {
  const s = new CardStore();
  s.push(card('a'));
  mock.timers.tick(CARD_MS - 1);
  assert.equal(s.items.length, 1);
  mock.timers.tick(1);
  assert.equal(s.items.length, 0);
});

test('hover pauses; letting go gives the rest, at least a second', () => {
  const s = new CardStore();
  const id = s.push(card('a'));
  mock.timers.tick(CARD_MS - 200);
  s.hold(id);
  mock.timers.tick(60_000);
  assert.equal(s.items.length, 1);
  s.release(id);
  mock.timers.tick(MIN_LEFT_MS - 1);
  assert.equal(s.items.length, 1);
  mock.timers.tick(1);
  assert.equal(s.items.length, 0);
});

test('dismissing stops its clock', () => {
  const s = new CardStore();
  const id = s.push(card('a'));
  s.dismiss(id);
  s.push(card('b'));
  mock.timers.tick(CARD_MS);
  assert.equal(s.items.length, 0);
});
