import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import type { ModButtons } from '../../ipc/bindings/ModButtons';
import { addTimeout, buttonKey, buttonOn, canAddButton, fullOrder, lineSlots, modButtonsFor, modSlots, retimeButton, toggleButton, type LineMod } from './lineMod.ts';

const buttons: ModButtons = { delete: true, timeouts: [60], warn: false, ban: true, unban: false, unban: false, order: ['delete', { timeout: 60 }, 'warn', 'ban'] };
// V107-R14 Rust's answers per role, written by cargo test (crates/moderation/src/may.rs).
const caps = JSON.parse(readFileSync(new URL('../../ipc/bindings/caps.fixture.json', import.meta.url), 'utf8'));
const mod: LineMod = { me: '1', caps: caps.moderator, inline: true, slots: modSlots(buttons) };
const user = (id: string, login: string, ...sets: string[]) => ({ id, login, badges: sets.map((set) => ({ set })) });

test('mod mode puts buttons on viewers only (V101-33)', () => {
  assert.equal(modButtonsFor(mod, user('2', 'viewer', 'subscriber'), 'chan'), true);
  assert.equal(modButtonsFor(mod, user('1', 'me'), 'chan'), false, 'own lines');
  assert.equal(modButtonsFor(mod, user('3', 'chan', 'broadcaster'), 'chan'), false, 'the broadcaster');
  assert.equal(modButtonsFor(mod, user('4', 'othermod', 'moderator'), 'chan'), false, 'another mod');
  assert.equal(modButtonsFor(mod, user('5', 'lead', 'lead_moderator'), 'chan'), false, 'a lead mod');
});

test('the broadcaster gets them on mods too', () => {
  const owner = { ...mod, caps: caps.broadcaster };
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
    unban: false,
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
    unban: false,
    order: ['ban', { timeout: 60 }, { timeout: 30 }, 'delete', 'warn'],
  };
  assert.deepEqual(fullOrder(b).map(buttonKey), ['ban', 't60', 't30', 'delete', 'warn', 't300', 'unban']);
});

test('no buttons, no mod mode on lines', () => {
  const none: ModButtons = { delete: false, timeouts: [], warn: false, ban: false, unban: false, order: [] };
  assert.deepEqual(modSlots(none), []);
  assert.equal(modButtonsFor({ ...mod, slots: [] }, user('2', 'viewer'), 'chan'), false);
});

test('V105-15 no mod button is locked: each one adds and removes (warn too)', () => {
  const base: ModButtons = { delete: false, timeouts: [], warn: false, ban: false, unban: false, order: ['delete', 'warn', 'ban'] };
  for (const x of ['delete', 'warn', 'ban', 'unban'] as const) {
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
  const b: ModButtons = { delete: true, timeouts: [], warn: false, ban: true, unban: false, unban: false, order: ['warn', 'ban', 'delete'] };
  const on = { ...b, ...toggleButton(b, 'warn') };
  assert.deepEqual(modSlots(on).map((s) => s.key), ['warn', 'ban', 'delete']);
  const off = { ...on, ...toggleButton(on, 'warn') };
  assert.deepEqual(fullOrder(off).map(buttonKey), ['warn', 'ban', 'delete', 'unban']);
});

test('V105-15 a removed timeout keeps its place and comes back there; a fifth is refused', () => {
  const b: ModButtons = { delete: true, timeouts: [60, 600], warn: true, ban: true, unban: false, unban: false, order: ['delete', { timeout: 60 }, { timeout: 600 }, 'warn', 'ban'] };
  const off = { ...b, ...toggleButton(b, { timeout: 60 }) };
  assert.deepEqual(off.timeouts, [600]);
  assert.deepEqual(fullOrder(off).map(buttonKey), ['delete', 't60', 't600', 'warn', 'ban', 'unban']);
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

test('V105-15 every preset gets a row in the order list, a fifth one off and not addable', () => {
  const b: ModButtons = { delete: true, timeouts: [5, 30, 60, 600], warn: true, ban: true, unban: false, unban: false, order: [] };
  const keys = fullOrder(b, [5, 30, 60, 600, 900]).map(buttonKey);
  assert.deepEqual(keys, ['delete', 't5', 't30', 't60', 't600', 'warn', 'ban', 'unban', 't900']);
  assert.equal(canAddButton(b, { timeout: 900 }), false);
  assert.deepEqual(modSlots(b).map((s) => s.key), ['delete', 't5', 't30', 't60', 't600', 'warn', 'ban']);
  assert.deepEqual(fullOrder(b).map(buttonKey), keys.filter((k) => k !== 't900'));
});

test('V107-R22 every mod role gets buttons on viewers, from its own channel too', () => {
  for (const role of ['moderator', 'leadModerator', 'broadcaster']) {
    const m = { ...mod, caps: caps[role] };
    assert.equal(modButtonsFor(m, user('2', 'viewer'), 'chan'), true, role);
    assert.equal(modButtonsFor(m, user('6', 'vip', 'vip', 'subscriber'), 'chan'), true, role + ' on a VIP');
    assert.equal(modButtonsFor(m, user('1', 'me'), 'chan'), false, role + ' own lines');
  }
  // The broadcaster in their own channel: their own lines are the channel's, viewers' get them.
  const owner = { ...mod, me: '9', caps: caps.broadcaster };
  assert.equal(modButtonsFor(owner, user('9', 'chan', 'broadcaster'), 'chan'), false);
  assert.equal(modButtonsFor(owner, user('2', 'viewer'), 'chan'), true);
  // V107-R22b Lead mods get them on other mods too; plain mods don't (first test).
  assert.equal(modButtonsFor({ ...owner, caps: caps.leadModerator }, user('4', 'othermod', 'moderator'), 'chan'), true);
  assert.equal(modButtonsFor({ ...owner, caps: caps.leadModerator }, user('5', 'lead', 'lead_moderator'), 'chan'), true);
});

test('V107-R22 a typed timeout goes on as a button, last; a fifth or a twin is refused', () => {
  assert.deepEqual(addTimeout(buttons, 420), {
    timeouts: [60, 420],
    order: ['delete', { timeout: 60 }, 'warn', 'ban', 'unban', { timeout: 420 }],
  });
  assert.equal(typeof addTimeout(buttons, 60), 'string');
  assert.equal(typeof addTimeout({ ...buttons, timeouts: [1, 2, 3, 4] }, 5), 'string');
});

test('V107-R22 a timeout button takes another length in its place', () => {
  const b: ModButtons = { ...buttons, timeouts: [60, 600], order: ['ban', { timeout: 600 }, 'delete', { timeout: 60 }, 'warn'] };
  assert.deepEqual(retimeButton(b, 600, 420), {
    timeouts: [60, 420],
    order: ['ban', { timeout: 420 }, 'delete', { timeout: 60 }, 'warn', 'unban'],
  });
  assert.deepEqual(retimeButton(b, 60, 60), {});
  assert.equal(typeof retimeButton(b, 60, 600), 'string', 'a length with a button already');
  // An unpicked preset row: changed in place and stays off.
  const r = retimeButton(buttons, 3600, 7200, [60, 3600]);
  assert.ok(typeof r !== 'string');
  assert.deepEqual(r.timeouts, [60]);
  assert.deepEqual(r.order, ['delete', { timeout: 60 }, 'warn', 'ban', 'unban', { timeout: 7200 }]);
});

test('V107-R22b Unban: off by default, after Ban, dragged like the others', () => {
  assert.ok(!modSlots(buttons).some((s) => s.kind === 'unban'));
  assert.equal(buttonOn(buttons, 'unban'), false);
  const on = { ...buttons, ...toggleButton(buttons, 'unban') };
  assert.equal(on.unban, true);
  assert.deepEqual(modSlots(on).map((s) => s.key), ['delete', 't60', 'ban', 'unban']);
  const moved: ModButtons = { ...on, order: ['unban', 'delete', { timeout: 60 }, 'warn', 'ban'] };
  assert.deepEqual(modSlots(moved).map((s) => s.key), ['unban', 'delete', 't60', 'ban']);
  assert.deepEqual(fullOrder(buttons).map(buttonKey), ['delete', 't60', 'warn', 'ban', 'unban'], 'an order from before Unban');
});

test("V107-R22b a timed-out sender's older lines keep only Unban", () => {
  const slots = modSlots({ ...buttons, unban: true });
  assert.deepEqual(lineSlots(slots, true).map((s) => s.key), ['unban']);
  assert.deepEqual(lineSlots(slots, false), slots);
  assert.deepEqual(lineSlots(modSlots(buttons), true), [], 'Unban off: nothing on deleted lines');
});
