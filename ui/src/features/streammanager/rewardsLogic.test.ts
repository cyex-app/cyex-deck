import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { SmReward } from '../../ipc/bindings/SmReward';
import { blank, facts, rewardOptions, sameEdit, toEdit, waiting } from './rewardsLogic.ts';

const R: SmReward = {
  id: 'a',
  title: 'Hydrate',
  prompt: '',
  cost: 1500,
  color: '#145AFF',
  image: null,
  enabled: true,
  paused: false,
  inStock: true,
  subOnly: false,
  inputRequired: false,
  skipQueue: false,
  cooldown: null,
  maxPerStream: null,
  maxPerUser: null,
  cooldownUntil: null,
  redeemedThisStream: null,
  waiting: 0,
  waitingMore: false,
};

test('a row says only what is unusual', () => {
  assert.deepEqual(facts(R), []);
  assert.deepEqual(facts({ ...R, enabled: false, paused: true, inStock: false }), ['Off']);
  assert.deepEqual(
    facts({ ...R, paused: true, cooldown: 300, maxPerStream: 1000, maxPerUser: 2, inputRequired: true, skipQueue: true }),
    ['Paused', '5m cooldown', '1.000 per stream', '2 per viewer', 'Asks for text', 'Skips the queue'],
  );
  const now = Date.parse('2026-10-21T10:00:00Z');
  assert.deepEqual(facts({ ...R, cooldown: 60, cooldownUntil: '2026-10-21T10:01:00Z' }, now), ['Cooling down']);
});

test('waiting counts', () => {
  assert.equal(waiting(R), null);
  assert.equal(waiting({ ...R, waiting: 1200 }), '1.200 waiting');
  assert.equal(waiting({ ...R, waiting: 50, waitingMore: true }), '50+ waiting');
});

test('edits compare field by field', () => {
  const e = toEdit(R);
  assert.ok(sameEdit(e, toEdit(R)));
  assert.ok(!sameEdit(e, { ...e, cost: 1 }));
  assert.equal(blank().cost, 100);
  assert.equal(blank().title, '');
});

test('V107-R43 the reward filter lists every reward with what waits', () => {
  const o = rewardOptions([R, { ...R, id: 'b', title: 'Song', waiting: 50, waitingMore: true }, { ...R, id: 'c', title: 'Quiet', waiting: 0 }]);
  assert.deepEqual(o[0], { value: '', label: 'All rewards' });
  assert.equal(o[2].label, 'Song (50+)');
  assert.equal(o[3].label, 'Quiet');
  assert.equal(o.length, 4);
});
