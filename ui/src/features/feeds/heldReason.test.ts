import assert from 'node:assert/strict';
import { test } from 'node:test';

import { heldLabel } from './heldReason.ts';

test('V108-N15 heldLabel names links and terms', () => {
  assert.equal(heldLabel('blocked_link'), 'Blocked link');
  assert.equal(heldLabel('blocked_term:bad: word'), 'Blocked term · bad: word');
  assert.equal(heldLabel('blocked_term'), 'Blocked term');
  assert.equal(heldLabel('blocked_term:  '), 'Blocked term');
});

test('V112-28 heldLabel lists several terms, capped', () => {
  assert.equal(heldLabel('blocked_term:bad\u001fworse'), 'Blocked term · bad, worse');
  assert.equal(heldLabel('blocked_term:a\u001fb\u001fc\u001fd\u001fe'), 'Blocked term · a, b, c, …');
  assert.equal(heldLabel('blocked_term:\u001f \u001f'), 'Blocked term');
  assert.equal(heldLabel(`blocked_term:${'x'.repeat(40)}`), `Blocked term · ${'x'.repeat(31)}…`);
});

test('V108-N15 heldLabel shows category and level', () => {
  assert.equal(heldLabel('automod:swearing:3'), 'Swearing · 3');
  assert.equal(heldLabel('automod:sexuality_sex_or_gender'), 'Sexuality sex or gender');
});

test('V108-N15 heldLabel falls back for older payloads', () => {
  assert.equal(heldLabel('AutoMod'), 'AutoMod');
  assert.equal(heldLabel(''), 'AutoMod');
  assert.equal(heldLabel('aggressive'), 'Aggressive');
});
