import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { CyexMe } from '../../ipc/bindings/CyexMe.ts';
import { badgeTitle, choiceOf, isCyexBadge, nothingOwned, owns, plusNote, shownBadges, usercardLine } from './cyexBadges.ts';

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

test('V109-B02 the Supporter+ note and the usercard line', () => {
  assert.equal(plusNote(me()), 'No badge yet');
  const pending = me({ plus: { active: true, source: null, until: null, renewsAt: null, badge: { status: 'pending' }, live: null } });
  assert.equal(plusNote(pending), 'Waiting for review');
  assert.equal(usercardLine(['7TV Sub', 'cyex founder #37', 'cyex supporter+']), 'cyex founder #37 · supporter+');
  assert.equal(usercardLine(['FFZ Bot']), '');
});
