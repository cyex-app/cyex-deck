import assert from 'node:assert/strict';
import { test } from 'node:test';

import { tallyDetail, tallyParts } from './massTally.ts';

const t = { done: 1200, already: 3, gone: 40, protected: 1, throttled: 0, other: 2, codes: ['X'], retries: 5 };

test('V107-R21 the outcome line skips empty buckets', () => {
  assert.deepEqual(tallyParts(t, 'ban'), [
    '1.200 banned',
    '3 already banned',
    '40 not found or suspended',
    '1 mod or broadcaster',
    '2 failed',
  ]);
  assert.equal(tallyParts({ ...t, already: 0 }, 'unban')[0], '1.200 unbanned');
  assert.equal(tallyParts(t, 'unban')[1], '3 weren’t banned');
  assert.equal(tallyParts({ ...t, already: 1 }, 'unban')[1], '1 wasn’t banned');
  assert.equal(tallyParts(t, 'timeout')[0], '1.200 timed out');
});

test('V107-R21 the tooltip names codes and retries', () => {
  assert.equal(tallyDetail(t), 'Failed: X. 5 sent again after Twitch slowed down');
  assert.equal(tallyDetail({ ...t, codes: [], retries: 0 }), '');
});
