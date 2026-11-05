import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { CyexMe } from '../../ipc/bindings/CyexMe.ts';
import {
  BADGE_PAGE,
  badgeOptions,
  badgeTitle,
  choiceOf,
  day,
  isCyexBadge,
  nothingOwned,
  nothingToSet,
  owns,
  plusState,
  shownBadges,
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

test('V109-B03 the badge picker: owned ones pick, the rest stay off', () => {
  const none = badgeOptions(me());
  assert.deepEqual(none.map((o) => [o.value, o.label, o.owned]), [
    ['founder', 'Founder', false],
    ['supporter', 'Supporter', false],
    ['none', 'None', true],
  ]);
  const all = badgeOptions(me({ badges: [{ id: 'founder', number: 1234 }, { id: 'supporter', number: null }] }));
  assert.equal(all[0].label, 'Founder #1.234');
  assert.ok(all.every((o) => o.owned));
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
