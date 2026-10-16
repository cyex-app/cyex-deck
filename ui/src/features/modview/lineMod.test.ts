import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ModButtons } from '../../ipc/bindings/ModButtons';
import { buttonKey, buttonOn, canAddButton, fullOrder, modButtonsFor, modSlots, toggleButton, type LineMod } from './lineMod.ts';

const buttons: ModButtons = { delete: true, timeouts: [60], warn: false, ban: true, order: ['delete', { timeout: 60 }, 'warn', 'ban'] };
const mod: LineMod = { me: '1', broadcaster: false, inline: true, slots: modSlots(buttons) };
const user = (id: string, login: string, ...sets: string[]) => ({ id, login, badges: sets.map((set) => ({ set })) });

test('mod mode puts buttons on viewers only (V101-33)', () => {
  assert.equal(modButtonsFor(mod, user('2', 'viewer', 'subscriber'), 'chan'), true);
  assert.equal(modButtonsFor(mod, user('1', 'me'), 'chan'), false, 'own lines');
  assert.equal(modButtonsFor(mod, user('3', 'chan', 'broadcaster'), 'chan'), false, 'the broadcaster');
  assert.equal(modButtonsFor(mod, user('4', 'othermod', 'moderator'), 'chan'), false, 'another mod');
  assert.equal(modButtonsFor(mod, user('5', 'lead', 'lead_moderator'), 'chan'), false, 'a lead mod');
});

test('the broadcaster gets them on mods too', () => {
  const owner = { ...mod, broadcaster: true };
  assert.equal(modButtonsFor(owner, user('4', 'othermod', 'moderator'), 'chan'), true);
});

test('nothing with mod mode off or without a mod role', () => {
  assert.equal(modButtonsFor({ ...mod, inline: false }, user('2', 'viewer'), 'chan'), false);
  assert.equal(modButtonsFor(null, user('2', 'viewer'), 'chan'), false);
  assert.equal(modButtonsFor(mod, null, 'chan'), false);
});

test('V105-15 buttons follow the order, each timeout on its own', () => {
  const b: ModButtons = {
    delete: true,
    timeouts: [60, 600],
    warn: false,
    ban: true,
    order: [{ timeout: 600 }, 'ban', 'delete', { timeout: 60 }, 'warn'],
  };
  const slots = modSlots(b);
  assert.deepEqual(slots.map((s) => s.key), ['t600', 'ban', 'delete', 't60']);
  assert.ok(slots.filter((s) => s.kind === 'timeout').every((s) => s.kind === 'timeout' && s.short));
  const one = modSlots({ ...b, timeouts: [60], order: ['delete', { timeout: 60 }, 'warn', 'ban'] });
  const t = one.find((s) => s.kind === 'timeout');
  assert.ok(t && t.kind === 'timeout' && !t.short);
});

test('V105-15 a just-picked timeout goes last; an unpicked one keeps its place', () => {
  const b: ModButtons = {
    delete: true,
    timeouts: [60, 300],
    warn: false,
    ban: true,
    order: ['ban', { timeout: 60 }, { timeout: 30 }, 'delete', 'warn'],
  };
  assert.deepEqual(fullOrder(b).map(buttonKey), ['ban', 't60', 't30', 'delete', 'warn', 't300']);
});

test('no buttons, no mod mode on lines', () => {
  const none: ModButtons = { delete: false, timeouts: [], warn: false, ban: false, order: [] };
  assert.deepEqual(modSlots(none), []);
  assert.equal(modButtonsFor({ ...mod, slots: [] }, user('2', 'viewer'), 'chan'), false);
});

test('V105-15 no mod button is locked: each one adds and removes (warn too)', () => {
  const base: ModButtons = { delete: false, timeouts: [], warn: false, ban: false, order: ['delete', 'warn', 'ban'] };
  for (const x of ['delete', 'warn', 'ban'] as const) {
    const on = { ...base, ...toggleButton(base, x) };
    assert.equal(on[x], true, `${x} adds`);
    assert.ok(modSlots(on).some((s) => s.key === x));
    const off = { ...on, ...toggleButton(on, x) };
    assert.equal(off[x], false, `${x} removes`);
    assert.ok(!modSlots(off).some((s) => s.key === x));
  }
  const t = { ...base, ...toggleButton(base, { timeout: 600 }) };
  assert.deepEqual(t.timeouts, [600]);
  assert.deepEqual({ ...t, ...toggleButton(t, { timeout: 600 }) }.timeouts, []);
});

test('V105-15 warn keeps its place in the order when added and removed', () => {
  const b: ModButtons = { delete: true, timeouts: [], warn: false, ban: true, order: ['warn', 'ban', 'delete'] };
  const on = { ...b, ...toggleButton(b, 'warn') };
  assert.deepEqual(modSlots(on).map((s) => s.key), ['warn', 'ban', 'delete']);
  const off = { ...on, ...toggleButton(on, 'warn') };
  assert.deepEqual(fullOrder(off).map(buttonKey), ['warn', 'ban', 'delete']);
});

test('V105-15 a removed timeout keeps its place and comes back there; a fifth is refused', () => {
  const b: ModButtons = { delete: true, timeouts: [60, 600], warn: true, ban: true, order: ['delete', { timeout: 60 }, { timeout: 600 }, 'warn', 'ban'] };
  const off = { ...b, ...toggleButton(b, { timeout: 60 }) };
  assert.deepEqual(off.timeouts, [600]);
  assert.deepEqual(fullOrder(off).map(buttonKey), ['delete', 't60', 't600', 'warn', 'ban']);
  assert.deepEqual(modSlots(off).map((s) => s.key), ['delete', 't600', 'warn', 'ban']);
  const back = { ...off, ...toggleButton(off, { timeout: 60 }) };
  assert.deepEqual(modSlots(back).map((s) => s.key), ['delete', 't60', 't600', 'warn', 'ban']);
  const full: ModButtons = { ...b, timeouts: [5, 30, 60, 600], order: [...b.order, { timeout: 5 }, { timeout: 30 }, { timeout: 900 }] };
  assert.equal(canAddButton(full, { timeout: 900 }), false);
  assert.equal(canAddButton(full, { timeout: 60 }), true, 'a timeout that is on can be removed');
  assert.equal(canAddButton(full, 'ban'), true);
  assert.deepEqual(toggleButton(full, { timeout: 900 }), {});
});

test('a removed timeout is off in the order list', () => {
  const b = { delete: true, warn: false, ban: true, timeouts: [60], order: [] } as unknown as Parameters<typeof fullOrder>[0];
  assert.equal(buttonOn(b, { timeout: 60 }), true);
  assert.equal(buttonOn(b, { timeout: 600 }), false);
  assert.equal(buttonOn(b, 'warn'), false);
  assert.equal(buttonOn(b, 'ban'), true);
});
