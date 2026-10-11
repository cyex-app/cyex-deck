import assert from 'node:assert/strict';
import { test } from 'node:test';

import { pinText } from './pinSend.ts';

test('/pin with text sends and pins; with a message id it pins that (M11)', () => {
  assert.equal(pinText('/pin Giveaway at 8, type !join'), 'Giveaway at 8, type !join');
  assert.equal(pinText('  /pin   hi  '), 'hi');
  assert.equal(pinText('/pin bddd8905-c104-4f7a-9c2e-0a1b2c3d4e5f 5m'), null);
  assert.equal(pinText('/pin'), null);
  assert.equal(pinText('/pinned hi'), null);
  assert.equal(pinText('hello /pin x'), null);
});
