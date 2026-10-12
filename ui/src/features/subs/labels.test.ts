import assert from 'node:assert/strict';
import { test } from 'node:test';

import { canStop, months, standing, tierLabel } from './labels.ts';

const base = { renewsAt: null, endsAt: null, prime: false, gifter: null, canceled: false, platform: 'WEB' };

test('tiers', () => {
  assert.equal(tierLabel('1000'), 'Tier 1');
  assert.equal(tierLabel('3000'), 'Tier 3');
  assert.equal(tierLabel(''), 'Sub');
});

test('where a sub stands', () => {
  assert.equal(standing({ ...base, renewsAt: '2026-11-01T12:00:00Z' }), 'Renews 2026/11/01');
  assert.equal(standing({ ...base, canceled: true, endsAt: '2026-11-01T12:00:00Z' }), 'Ends 2026/11/01');
  assert.equal(standing({ ...base, gifter: 'G' }), 'Gift from G');
  assert.equal(standing({ ...base, prime: true }), 'Prime');
});

test('only paid web subs that renew can be stopped here', () => {
  assert.ok(canStop({ ...base, renewsAt: 'x' }));
  assert.ok(!canStop({ ...base, renewsAt: 'x', platform: 'IOS' }));
  assert.ok(!canStop({ ...base, renewsAt: 'x', gifter: 'g' }));
  assert.ok(!canStop({ ...base }));
});

test('months', () => {
  assert.equal(months(1), '1 month');
  assert.equal(months(14), '14 months');
  assert.equal(months(null), '');
});
