import assert from 'node:assert/strict';
import { test } from 'node:test';

import { afterAction, shown } from './optimistic.ts';

test('afterAction sets a timeout end, a ban, a lift, or nothing', () => {
  const t = afterAction('timeout', 'bob 60', 1_000_000, 'cyexbot');
  assert.deepEqual(t, { until: new Date(1_060_000).toISOString(), by: 'cyexbot', reason: '' });
  assert.deepEqual(afterAction('ban', 'bob', 0, 'me'), { until: null, by: 'me', reason: '' });
  assert.equal(afterAction('unban', 'bob', 0, 'me'), null);
  assert.equal(afterAction('untimeout', 'bob', 0, 'me'), null);
  assert.equal(afterAction('warn', 'bob why', 0, 'me'), undefined);
  assert.equal(afterAction('timeout', 'bob x', 0, 'me'), undefined);
});

test('shown keeps the local state until the log agrees or ages out', () => {
  const ban = { until: null, by: 'me', reason: '' };
  const local = { ban, at: 0 };
  assert.equal(shown(null, undefined, 0), undefined);
  assert.equal(shown(local, undefined, 1), ban);
  assert.equal(shown(local, null, 1000), ban);
  assert.equal(shown(local, null, 9000), null);
  const logged = { until: null, by: 'x', reason: 'r' };
  assert.equal(shown(local, logged, 1000), logged);
  assert.equal(shown({ ban: null, at: 0 }, logged, 1000), null);
});
