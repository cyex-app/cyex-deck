import assert from 'node:assert/strict';
import { test } from 'node:test';

import { massEnd, tallyDetail, tallyParts } from './massTally.ts';

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

test('V107-H3b Stop after the last send reads as done', () => {
  assert.deepEqual(massEnd(3, 4, 1000, true), { line: 'Stopped. 3 of 1.000 done.', stopped: true });
  assert.deepEqual(massEnd(8, 10, 10, true), { line: 'Done: 8 of 10.', stopped: false });
  assert.deepEqual(massEnd(4, 6, 10, false), { line: 'Done: 4 of 10.', stopped: false });
});
