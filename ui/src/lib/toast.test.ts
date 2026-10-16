import assert from 'node:assert/strict';
import { mock, test } from 'node:test';

// The store is a .svelte.ts module: outside the compiler, $state is the plain value.
(globalThis as unknown as { $state: <T>(v: T) => T }).$state = (v) => v;
const { ACTION_MS, LIFETIME_MS, MAX_VISIBLE, ToastStore } = await import('./toast.svelte.ts');

mock.timers.enable({ apis: ['setTimeout', 'Date'] });

test('the same text bumps a count instead of stacking', () => {
  const t = new ToastStore();
  t.error('Couldn’t reach Twitch.');
  t.error('Couldn’t reach Twitch.');
  t.info('Couldn’t reach Twitch.');
  assert.equal(t.items.length, 2);
  assert.equal(t.items[0].count, 2);
  assert.equal(t.items[1].count, 1);
});

test('each side keeps the newest 3', () => {
  const t = new ToastStore();
  for (let i = 0; i < 5; i++) t.info(`n${i}`);
  t.error('e');
  const notes = t.items.filter((x) => x.kind !== 'error').map((x) => x.text);
  assert.deepEqual(notes, ['n2', 'n3', 'n4']);
  assert.equal(notes.length, MAX_VISIBLE);
  assert.equal(t.items.filter((x) => x.kind === 'error').length, 1);
});

test('toasts go after their time; errors stay longer', () => {
  const t = new ToastStore();
  t.info('i');
  t.error('e');
  mock.timers.tick(LIFETIME_MS.info);
  assert.deepEqual(t.items.map((x) => x.text), ['e']);
  mock.timers.tick(LIFETIME_MS.error - LIFETIME_MS.info);
  assert.equal(t.items.length, 0);
});

test('a repeat restarts the time', () => {
  const t = new ToastStore();
  t.info('i');
  mock.timers.tick(LIFETIME_MS.info - 100);
  t.info('i');
  mock.timers.tick(200);
  assert.equal(t.items.length, 1);
  mock.timers.tick(LIFETIME_MS.info);
  assert.equal(t.items.length, 0);
});

test('a held toast waits, then runs out the rest of its time', () => {
  const t = new ToastStore();
  t.info('i');
  const id = t.items[0].id;
  mock.timers.tick(1000);
  t.hold(id);
  mock.timers.tick(60_000);
  assert.equal(t.items.length, 1);
  t.release(id);
  mock.timers.tick(LIFETIME_MS.info - 1000 - 1);
  assert.equal(t.items.length, 1);
  mock.timers.tick(1);
  assert.equal(t.items.length, 0);
});

test('dismiss removes it at once', () => {
  const t = new ToastStore();
  t.success('Saved.');
  t.dismiss(t.items[0].id);
  assert.equal(t.items.length, 0);
});

test('a toast button runs once and closes the toast; it keeps the toast up longer', () => {
  const t = new ToastStore();
  let undone = 0;
  t.info('Blocked kuni.', { action: { label: 'Undo', run: () => undone++ } });
  mock.timers.tick(LIFETIME_MS.info);
  assert.equal(t.items.length, 1);
  t.act(t.items[0].id);
  assert.equal(undone, 1);
  assert.equal(t.items.length, 0);
  t.info('x', { action: { label: 'Undo', run: () => {} } });
  mock.timers.tick(ACTION_MS);
  assert.equal(t.items.length, 0);
});
