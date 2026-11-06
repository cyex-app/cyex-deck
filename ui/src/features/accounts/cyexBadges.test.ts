import assert from 'node:assert/strict';
import { test } from 'node:test';

import { type MenuItem, menuProblems } from '../../components/menuItems.ts';
import type { CyexMe } from '../../ipc/bindings/CyexMe.ts';
import type { CyexView } from '../../ipc/bindings/CyexView.ts';
import {
  BADGE_PAGE,
  badgeTitle,
  choiceOf,
  day,
  isCyexBadge,
  nothingOwned,
  nothingToSet,
  owns,
  plusGate,
  plusState,
  shownBadges,
  SLOT1_KINDS,
  slot1Current,
  slot1Menu,
  slot1Options,
  slot2Menu,
  STORE,
  usercardLine,
} from './cyexBadges.ts';

const pic = (n: string) => ({ '1x': `${n}1`, '2x': `${n}2`, '4x': `${n}4` });

function me(over: Partial<CyexMe> = {}): CyexMe {
  return {
    twitch: { id: '1', login: 'a', displayName: 'A', avatar: null },
    badges: [],
    plus: { active: false, source: null, until: null, renewsAt: null, badge: null, live: null },
    equipped: { main: null, custom: false, shown: [] },
    ...over,
  };
}

test('V109-B02 tooltips', () => {
  assert.equal(badgeTitle('founder', 37), 'cyex founder #37');
  assert.equal(badgeTitle('founder'), 'cyex founder');
  assert.equal(badgeTitle('supporter'), 'cyex supporter');
  assert.equal(badgeTitle('custom'), 'cyex supporter+');
  assert.ok(isCyexBadge('cyex:founder-37') && !isCyexBadge('homies:a0') && !isCyexBadge('cyex'));
});

test('V109-B02 a founder counts only with its number', () => {
  const m = me({ badges: [{ id: 'founder', number: null }, { id: 'supporter', number: null }] });
  assert.ok(!owns(m, 'founder') && owns(m, 'supporter') && !owns(m, 'custom'));
  assert.ok(!nothingOwned(m));
  assert.ok(nothingOwned(me({ badges: [{ id: 'founder', number: null }] })));
  assert.equal(choiceOf(me({ equipped: { main: 'other', custom: false, shown: [] } })), 'none');
});

test('V109-B02 the preview shows what is shown, two at most, in order', () => {
  const m = me({
    badges: [{ id: 'founder', number: 7 }, { id: 'supporter', number: null }],
    plus: { active: true, source: 'paddle', until: null, renewsAt: null, badge: null, live: pic('c') },
    equipped: { main: 'founder', custom: true, shown: ['custom', 'founder', 'supporter'] },
  });
  const got = shownBadges(m, pic('f'), pic('s'));
  assert.deepEqual(got.map((b) => b.title), ['cyex supporter+', 'cyex founder #7']);
  assert.equal(got[0].pics['2x'], 'c2');
  assert.deepEqual(shownBadges(me({ equipped: { main: null, custom: false, shown: [] } }), pic('f'), pic('s')), []);
  // Shown but not owned (stale): skipped.
  assert.deepEqual(shownBadges(me({ equipped: { main: 'founder', custom: true, shown: ['founder', 'custom'] } }), pic('f'), pic('s')), []);
});

test('V109-B02 the usercard line', () => {
  assert.equal(usercardLine(['7TV Sub', 'cyex founder #37', 'cyex supporter+']), 'cyex founder #37 · supporter+');
  assert.equal(usercardLine(['FFZ Bot']), '');
});

const plus = (over: Partial<CyexMe['plus']>): CyexMe['plus'] => ({ active: true, source: 'paddle', until: null, renewsAt: null, badge: null, live: null, ...over });

const items = (list: ReturnType<typeof slot1Menu>) => list.filter((e): e is MenuItem => e !== 'separator');

test('V109-B04 slot 1 comes from the kinds list', () => {
  assert.deepEqual(SLOT1_KINDS.map((k) => k.id), ['founder', 'supporter']);
  assert.deepEqual(slot1Options(me()).map((o) => [o.id, o.label, o.owned]), [
    ['founder', 'Founder', false],
    ['supporter', 'Supporter', false],
  ]);
  const all = me({ badges: [{ id: 'founder', number: 1234 }, { id: 'supporter', number: null }], equipped: { main: 'supporter', custom: false, shown: [] } });
  assert.equal(slot1Options(all)[0].label, 'Founder #1.234');
  assert.equal(slot1Current(all)?.id, 'supporter');
  // Worn but not owned (stale): the slot is empty.
  assert.equal(slot1Current(me({ equipped: { main: 'founder', custom: false, shown: [] } })), null);
  const view: CyexView = { state: 'ready', me: all, founder: pic('f'), supporter: pic('s') };
  assert.equal(SLOT1_KINDS[1].pics(view)?.['1x'], 's1');
  assert.equal(SLOT1_KINDS[0].pics({ state: 'offline' }), null);
});

test('V109-B04 slot 1 menu: owned badges, None, store; the checked one again takes it off', () => {
  const calls: (string | null)[] = [];
  const act = { equip: (m: string | null) => calls.push(m), store: () => calls.push('store') };
  const m = me({ badges: [{ id: 'founder', number: 7 }], equipped: { main: 'founder', custom: false, shown: [] } });
  const list = slot1Menu(m, act);
  assert.deepEqual(menuProblems(list), []);
  const it = items(list);
  assert.deepEqual(it.map((i) => [i.label, i.checked]), [
    ['Founder #7', true],
    ['None', false],
    ['Get in store', undefined],
  ]);
  it[0].run!();
  it[1].run!();
  it[2].run!();
  assert.deepEqual(calls, [null, null, 'store']);
  // Nothing worn: None is checked; owning everything drops the store item.
  const both = me({ badges: [{ id: 'founder', number: 7 }, { id: 'supporter', number: null }] });
  const it2 = items(slot1Menu(both, act));
  assert.deepEqual(it2.map((i) => [i.label, i.checked]), [
    ['Founder #7', false],
    ['Supporter', false],
    ['None', true],
  ]);
  it2[1].run!();
  assert.equal(calls.at(-1), 'supporter');
});

test('V109-B04 slot 2 menu: the picture link, then the switch', () => {
  const shown: boolean[] = [];
  const opened: string[] = [];
  const m = me({ plus: plus({ live: pic('c') }), equipped: { main: null, custom: true, shown: ['custom'] } });
  const list = slot2Menu(m, { show: (on) => shown.push(on), open: (u) => opened.push(u) });
  assert.deepEqual(menuProblems(list), []);
  const it = items(list);
  assert.deepEqual(it.map((i) => [i.label, i.checked]), [
    ['Change picture', undefined],
    ['Show in chat', true],
  ]);
  it[0].run!();
  it[1].run!();
  assert.deepEqual(opened, [BADGE_PAGE]);
  assert.deepEqual(shown, [false]);
});

const ready = (active: boolean): CyexView => ({ state: 'ready', me: me({ plus: plus({ active }) }), founder: pic('f'), supporter: pic('s') });

test('V109-B04 slot 2 gate: a fresh look first, 30 s cache, failures stay shut', async () => {
  let t = 0;
  let calls = 0;
  let answer: () => Promise<CyexView> = async () => ready(true);
  const gate = plusGate(() => (calls++, answer()), () => t);
  const first = await gate.check();
  assert.equal(first.kind, 'open');
  assert.ok(first.kind !== 'error' && first.fetched);
  assert.equal(calls, 1);
  // Within 30 s: no new call, from the cache.
  t = 29_000;
  const cached = await gate.check();
  assert.deepEqual(cached, { kind: 'open', fetched: null });
  assert.equal(calls, 1);
  // Later the plan has ended: locked, not opened.
  t = 31_000;
  answer = async () => ready(false);
  assert.equal((await gate.check()).kind, 'locked');
  assert.equal(calls, 2);
  // Two clicks while one look runs: one call.
  t = 70_000;
  const [a, b] = await Promise.all([gate.check(), gate.check()]);
  assert.equal(calls, 3);
  assert.equal(a.kind, b.kind);
  // A failed look: an error, and the next click asks again.
  t = 110_000;
  answer = async () => Promise.reject(new Error('net'));
  const err = await gate.check();
  assert.equal(err.kind, 'error');
  answer = async () => ({ state: 'offline' });
  assert.equal((await gate.check()).kind, 'error');
  assert.equal(calls, 5);
  answer = async () => ({ state: 'noToken' });
  const nt = await gate.check();
  assert.ok(nt.kind === 'error' && nt.message.includes('Sign in'));
  // The editor's own load counts as a look.
  const g2 = plusGate(async () => (calls++, ready(true)), () => t);
  g2.seen(ready(false));
  assert.ok(g2.fresh());
  assert.equal((await g2.check()).kind, 'locked');
  assert.equal(calls, 6);
});

test('V109-B03 Supporter+ states: plan line, note and the one link', () => {
  assert.equal(plusState(me()).kind, 'never');
  assert.ok(nothingToSet(me()));
  const live = plusState(me({ plus: plus({ renewsAt: '2026-12-05T12:00:00Z', until: '2026-12-05T12:00:00Z', live: pic('c') }) }));
  assert.equal(live.kind, 'live');
  assert.equal(live.plan, `Renews ${day('2026-12-05T12:00:00Z')}`);
  assert.deepEqual(live.link, { label: 'Change picture', url: BADGE_PAGE });
  const code = plusState(me({ plus: plus({ source: 'code', until: '2026-12-05T12:00:00Z' }) }));
  assert.equal(code.kind, 'empty');
  assert.ok(code.plan.startsWith('Until 2026/12/0'));
  assert.equal(code.link?.label, 'Upload a picture');
  assert.equal(plusState(me({ plus: plus({ badge: { status: 'pending' } }) })).note, 'Waiting for review');
  assert.equal(plusState(me({ plus: plus({ badge: { status: 'rejected' } }) })).kind, 'rejected');
  const ended = me({ plus: plus({ active: false, until: '2026-10-05T12:00:00Z', badge: { status: 'approved' }, live: pic('c') }) });
  const e = plusState(ended);
  assert.equal(e.kind, 'ended');
  assert.ok(e.plan.startsWith('Ended 2026/10/0'));
  assert.deepEqual(e.link, { label: 'Renew', url: STORE });
  // An ended plan still has a row: not the empty store state.
  assert.ok(nothingOwned(ended) && !nothingToSet(ended));
  assert.equal(day('garbage-in'), 'garbage/in');
});
