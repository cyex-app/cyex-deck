import assert from 'node:assert/strict';
import { test } from 'node:test';

import { subHint, subTier } from './subTier.ts';

test('subTier reads the tier from the badge version (V109-F74)', () => {
  assert.equal(subTier('0'), null);
  assert.equal(subTier('3'), null);
  assert.equal(subTier('999'), null);
  assert.equal(subTier('2000'), 2);
  assert.equal(subTier('2024'), 2);
  assert.equal(subTier('3000'), 3);
  assert.equal(subTier('3006'), 3);
  assert.equal(subTier(undefined), null);
  assert.equal(subTier(''), null);
  assert.equal(subTier('abc'), null);
  assert.equal(subTier('20x0'), null);
});

test('subHint adds the tier for tier 2 and 3 only', () => {
  assert.equal(subHint('subscriber', '6', '7'), '7 months');
  assert.equal(subHint('subscriber', '2006', '7'), 'Tier 2 · 7 months');
  assert.equal(subHint('subscriber', '3012', '1'), 'Tier 3 · 1 month');
  assert.equal(subHint('subscriber', '2000', undefined), 'Tier 2');
  assert.equal(subHint('subscriber', '0', undefined), undefined);
  assert.equal(subHint('founder', '2000', '7'), '7 months');
});
