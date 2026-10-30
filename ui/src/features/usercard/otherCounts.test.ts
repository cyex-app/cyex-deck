import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { OtherModCount } from '../../ipc/bindings/OtherModCount.ts';
import { countText, sortCounts } from './otherCounts.ts';

const c = (login: string, bans: number, timeouts: number, warnings = 0): OtherModCount => ({
  login,
  name: login,
  bans,
  timeouts,
  warnings,
});

test('V108-N16 countText joins what there is', () => {
  assert.equal(countText(c('a', 2, 1)), '2 bans · 1 timeout');
  assert.equal(countText(c('a', 0, 3, 1)), '3 timeouts · 1 warning');
  assert.equal(countText(c('a', 1, 0)), '1 ban');
  // Grouped with dots, like every count in the app.
  assert.equal(countText(c('a', 1200, 0)), '1.200 bans');
});

test('V108-N16 sortCounts puts bans first', () => {
  assert.deepEqual(
    sortCounts([c('b', 0, 5), c('a', 1, 0), c('c', 1, 2)]).map((x) => x.login),
    ['c', 'a', 'b'],
  );
});
