import assert from 'node:assert/strict';
import { test } from 'node:test';

import { FollowerCounts, followersText } from './channelFollowers.ts';

test('V111-29 followers read with dots and a singular', () => {
  assert.equal(followersText(12345), '12.345 followers');
  assert.equal(followersText(1), '1 follower');
  assert.equal(followersText(1234567), '1.234.567 followers');
});

test('a channel is asked once, whatever the case', async () => {
  const asked: string[] = [];
  const c = new FollowerCounts(async (l) => (asked.push(l), 5));
  assert.equal(await c.get('XQC'), 5);
  assert.equal(await c.get('xqc'), 5);
  assert.deepEqual(asked, ['xqc']);
});

test('a failed or empty answer is asked again next time', async () => {
  let n = 0;
  const c = new FollowerCounts(async () => {
    if (++n === 1) throw new Error('down');
    return 7;
  });
  assert.equal(await c.get('a'), null);
  await Promise.resolve();
  assert.equal(await c.get('a'), 7);
  assert.equal(n, 2);
});
