import assert from 'node:assert/strict';
import { test } from 'node:test';

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
  kindLabel,
  kindOk,
  oneTimeKinds,
  slot1Current,
  slot1Pick,
  gridHeight,
  SEARCH_FROM,
  slot1Rows,
  slot1Missing,
  slot1Choice,
  filterRows,
  plusPanel,
  uploadRows,
  readSlot1,
  saveSlot1,
  STORE,
  usercardLine,
} from './cyexBadges.ts';

const pic = (n: string) => ({ '1x': `${n}1`, '2x': `${n}2`, '4x': `${n}4` });

/** The catalog as the view brings it (store order). */
const KINDS = [
  { id: 'founder', title: 'cyex founder', images: pic('f') },
  { id: 'supporter', title: 'cyex supporter', images: pic('s') },
];
const viewOf = (m: CyexMe, kinds = KINDS): CyexView => ({ state: 'ready', me: m, kinds });

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
  assert.equal(choiceOf(me({ equipped: { main: 'Bad Id', custom: false, shown: [] } })), 'none');
});

test('V109-B02 the preview shows what is shown, two at most, in order', () => {
  const m = me({
    badges: [{ id: 'founder', number: 7 }, { id: 'supporter', number: null }],
    plus: { active: true, source: 'paddle', until: null, renewsAt: null, badge: null, live: pic('c') },
    equipped: { main: 'founder', custom: true, shown: ['custom', 'founder', 'supporter'] },
  });
  const got = shownBadges(m, viewOf(m));
  assert.deepEqual(got.map((b) => b.title), ['cyex supporter+', 'cyex founder #7']);
  assert.equal(got[0].pics['2x'], 'c2');
  assert.deepEqual(shownBadges(me({ equipped: { main: null, custom: false, shown: [] } }), { state: 'offline' }), []);
  // Shown but not owned (stale): skipped.
  const stale = me({ equipped: { main: 'founder', custom: true, shown: ['founder', 'custom'] } });
  assert.deepEqual(shownBadges(stale, viewOf(stale)), []);
});

test('V109-B02 the usercard line', () => {
  assert.equal(usercardLine(['7TV Sub', 'cyex founder #37', 'cyex supporter+']), 'cyex founder #37 · supporter+');
  assert.equal(usercardLine(['FFZ Bot']), '');
});

const plus = (over: Partial<CyexMe['plus']>): CyexMe['plus'] => ({ active: true, source: 'paddle', until: null, renewsAt: null, badge: null, live: null, ...over });

test('V109-B10 slot 1 comes from the catalog, any owned kind, numbered or not', () => {
  const all = me({ badges: [{ id: 'founder', number: 1234 }, { id: 'supporter', number: null }], equipped: { main: 'supporter', custom: false, shown: [] } });
  assert.equal(slot1Current(all), 'supporter');
  // Worn but not owned (stale): the slot is empty.
  assert.equal(slot1Current(me({ equipped: { main: 'founder', custom: false, shown: [] } })), null);
  const streak = { id: 'streak-1y', title: 'cyex 1 year streak', images: pic('y') };
  const m = me({
    badges: [{ id: 'streak-1y', number: 4 }, { id: 'founder', number: 37 }, { id: 'gold', number: null }],
    equipped: { main: 'streak-1y', custom: false, shown: ['streak-1y'] },
  });
  const v = viewOf(m, [...KINDS, streak]);
  // Store order first; an owned kind the catalog doesn't name yet goes last with a readable name.
  assert.deepEqual(oneTimeKinds(m, v).map((k) => k.id), ['founder', 'supporter', 'streak-1y', 'gold']);
  assert.deepEqual(slot1Rows(m, v).map((r) => r.name), ['Founder #37', '1 year streak #4', 'Gold']);
  assert.equal(slot1Current(m), 'streak-1y');
  assert.deepEqual(shownBadges(m, v).map((b) => b.title), ['cyex 1 year streak #4']);
  assert.equal(badgeTitle('gold', 2), 'cyex gold #2');
  assert.equal(badgeTitle('streak-1y', null, ' '), 'cyex streak 1y');
  assert.equal(kindLabel(m, streak), '1 year streak #4');
  assert.ok(slot1Missing(m, v), 'supporter not owned');
  assert.deepEqual(slot1Pick(m, 'gold'), { main: 'gold' });
  assert.ok(kindOk('streak-1y') && !kindOk('custom') && !kindOk('Bad Id') && !kindOk(''));
  assert.equal(choiceOf(me({ equipped: { main: 'gold', custom: false, shown: [] } })), 'gold');
});

test('V109-B05 slot 1 picker: owned cards in kinds order, search, store link', () => {
  const both = me({ badges: [{ id: 'supporter', number: null }, { id: 'founder', number: 7 }] });
  const view = viewOf(both);
  const rows = slot1Rows(both, view);
  assert.deepEqual(rows.map((r) => [r.item.def.id, r.name, r.usable]), [
    ['founder', 'Founder #7', true],
    ['supporter', 'Supporter', true],
  ]);
  assert.deepEqual(rows[0].item.images, ['f1', 'f2', 'f4', 'f4']);
  assert.deepEqual(filterRows(rows, ' sup ').map((r) => r.name), ['Supporter']);
  assert.equal(filterRows(rows, '').length, 2);
  assert.ok(!slot1Missing(both, view));
  const one = me({ badges: [{ id: 'founder', number: 7 }] });
  assert.deepEqual(slot1Rows(one, { state: 'offline' }).map((r) => r.item.images), [['', '', '', '']]);
  assert.ok(slot1Missing(one, viewOf(one)));
});

test('V109-B05 slot 1 picks and the switch: picking turns on, None takes off, off keeps the choice', () => {
  const owned = { badges: [{ id: 'founder', number: 7 }, { id: 'supporter', number: null }] };
  const wearing = me({ ...owned, equipped: { main: 'founder', custom: true, shown: [] } });
  assert.deepEqual(slot1Pick(wearing, 'supporter'), { main: 'supporter' });
  assert.deepEqual(slot1Pick(wearing, null), { main: null });
  assert.equal(slot1Pick(wearing, 'founder'), null); // already worn
  assert.equal(slot1Pick(me({ equipped: { main: null, custom: false, shown: [] } }), 'founder'), null); // not owned
  const off = me({ ...owned, equipped: { main: null, custom: false, shown: [] } });
  assert.equal(slot1Choice(wearing, 'supporter'), 'founder');
  assert.equal(slot1Choice(off, 'supporter'), 'supporter');
  assert.equal(slot1Choice(off, null), null);
  assert.equal(slot1Choice(me(), 'founder'), null); // remembered but not owned
  assert.deepEqual(slot1Pick(off, 'supporter'), { main: 'supporter' });
  assert.equal(slot1Pick(off, null), null);
});

test('V109-B05 slot 1 choice is kept per account while off', () => {
  const store = new Map<string, string>();
  const g = globalThis as { localStorage?: unknown };
  g.localStorage = {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
  };
  try {
    saveSlot1('1', 'supporter');
    assert.equal(readSlot1('1'), 'supporter');
    assert.equal(readSlot1('2'), null);
    saveSlot1('1', null);
    assert.equal(readSlot1('1'), null);
  } finally {
    delete g.localStorage;
  }
  // No storage at all: nothing kept, nothing thrown.
  assert.equal(readSlot1('1'), null);
  saveSlot1('1', 'founder');
});

test('V109-B05 the Supporter+ panel', () => {
  const live = plusPanel(me({ plus: plus({ renewsAt: '2026-12-05T12:00:00Z', badge: { status: 'approved' }, live: pic('c') }) }));
  assert.equal(live.review, 'Approved');
  assert.equal(live.plan, `Active, renews ${day('2026-12-05T12:00:00Z')}`);
  assert.equal(live.picture?.['1x'], 'c1');
  assert.equal(live.link?.label, 'Change picture');
  const review = plusPanel(me({ plus: plus({ until: '2026-12-05T12:00:00Z', badge: { status: 'pending' } }) }));
  assert.equal(review.review, 'In review');
  assert.ok(review.plan.startsWith('Active until 2026/12/0'));
  assert.equal(plusPanel(me({ plus: plus({ badge: { status: 'rejected', reason: null } }) })).review, 'Not approved');
  assert.equal(plusPanel(me({ plus: plus({}) })).review, 'No picture yet');
  const ended = plusPanel(me({ plus: plus({ active: false, until: '2026-10-05T12:00:00Z' }) }));
  assert.ok(ended.plan.startsWith('Ended 2026/10/0'));
  assert.equal(ended.link?.label, 'Renew');
});

const ready = (active: boolean): CyexView => viewOf(me({ plus: plus({ active }) }));

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
  // V109-F76 what the real server sends someone who never bought Supporter+
  assert.equal(plusState(me({ plus: plus({ active: false, badge: { status: 'none' } }) })).kind, 'never');
  assert.ok(nothingToSet(me()));
  const live = plusState(me({ plus: plus({ renewsAt: '2026-12-05T12:00:00Z', until: '2026-12-05T12:00:00Z', live: pic('c') }) }));
  assert.equal(live.kind, 'live');
  assert.equal(live.plan, `Renews ${day('2026-12-05T12:00:00Z')}`);
  assert.deepEqual(live.link, { label: 'Change picture', url: BADGE_PAGE });
  const code = plusState(me({ plus: plus({ source: 'code', until: '2026-12-05T12:00:00Z' }) }));
  assert.equal(code.kind, 'empty');
  assert.ok(code.plan.startsWith('Free until 2026/12/0'));
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

test('V109-B06 the panel shows why a picture was turned down', () => {
  const r = plusPanel(me({ plus: plus({ badge: { status: 'rejected', reason: 'Too blurry.' } }) }));
  assert.equal(r.review, 'Not approved');
  assert.equal(r.reason, 'Too blurry.');
  assert.equal(plusPanel(me({ plus: plus({ badge: { status: 'removed', reason: ' ' } }) })).reason, 'Try another one.');
  const newer = plusPanel(me({ plus: plus({ live: pic('c'), badge: { status: 'rejected', reason: 'No text.' } }) }));
  assert.equal(newer.review, 'New picture not approved');
  assert.equal(newer.picture?.['1x'], 'c1');
  assert.equal(plusPanel(me({ plus: plus({ live: pic('c'), badge: { status: 'pending', reason: null } }) })).review, 'New picture in review');
  assert.equal(plusPanel(me({ plus: plus({ live: pic('c'), badge: { status: 'approved', reason: null } }) })).reason, '');
});

test('V109-B06 upload history rows', () => {
  const rows = uploadRows({
    live: 12,
    items: [
      { id: 14, images: pic('p'), status: 'rejected', reason: 'No text.', restorable: false, uploadedAt: '2026-11-03T12:00:00Z', reviewedAt: null },
      { id: 12, images: pic('a'), status: 'approved', reason: null, restorable: true, uploadedAt: '2026-10-20T12:00:00Z', reviewedAt: null },
      { id: 9, images: pic('b'), status: 'approved', reason: 'stale', restorable: true, uploadedAt: '2026-09-02T12:00:00Z', reviewedAt: null },
      { id: 5, images: null, status: 'pending', reason: null, restorable: false, uploadedAt: '2026-08-01T12:00:00Z', reviewedAt: null },
      { id: 3, images: null, status: 'odd', reason: null, restorable: false, uploadedAt: 'bad', reviewedAt: null },
    ],
  });
  assert.deepEqual(rows.map((r) => [r.id, r.status, r.reason, r.live, r.restorable]), [
    [14, 'Not approved', 'No text.', false, false],
    [12, 'Live', '', true, false],
    [9, 'Approved', '', false, true],
    [5, 'In review', '', false, false],
    [3, 'Odd', '', false, false],
  ]);
  assert.ok(rows[0].date.startsWith('2026/11/0'));
  // V109-B07 Not approved: no direct picture, its owner preview instead.
  assert.equal(rows[0].pics, null);
  assert.equal(rows[0].preview, 'p2');
  assert.equal(rows[1].preview, null);
  assert.equal(rows[1].pics?.['1x'], 'a1');
  assert.equal(rows[3].preview, null);
  assert.equal(rows[4].date, 'bad');
  assert.deepEqual(uploadRows({ items: [], live: null }), []);
});

test('V109-B08 the badge grid shows every row, four at most', () => {
  assert.equal(gridHeight(3, 600, true), 44); // three row cards fit one line at 600 px
  assert.equal(gridHeight(3, 400, true), 2 * 44 + 6); // two columns
  assert.equal(gridHeight(3, 0, true), 3 * 44 + 2 * 6); // before it's measured: one column
  assert.equal(gridHeight(40, 400, true), 4 * 44 + 3 * 6);
  assert.equal(gridHeight(3, 300, false), 76);
  assert.equal(SEARCH_FROM, 9);
  const both = me({ badges: [{ id: 'founder', number: 7 }, { id: 'supporter', number: null }] });
  assert.deepEqual(slot1Rows(both, { state: 'offline' }).map((r) => r.sub), ['One-time', 'One-time']);
});
