import assert from 'node:assert/strict';
import { test } from 'node:test';

import { wearingText } from './wearing.ts';

test('names the item', () => {
  assert.equal(wearingText('paint', 'p1', 'Sunset'), 'Wearing Sunset');
  assert.equal(wearingText('badge', 'b1', 'Gold Star'), 'Wearing Gold Star');
});
test('none', () => {
  assert.equal(wearingText('paint', null, undefined), 'No paint');
  assert.equal(wearingText('badge', null, undefined), 'No badge');
});
test('unknown falls back', () => {
  assert.equal(wearingText('paint', 'p1', undefined), 'Wearing it');
  assert.equal(wearingText(null, null, undefined), 'Wearing it');
});
