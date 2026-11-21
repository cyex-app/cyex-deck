import assert from 'node:assert/strict';
import { test } from 'node:test';
import { pickerTabs } from '../features/emotes/menu.ts';
import { GROUPS } from '../features/settings/emoteGroups.ts';
import { CYEXTV_READY, cyextvOn } from './cyextv.ts';

test('V112-23 a saved cyex.tv setting counts only once cyex.tv is ready', () => {
  assert.equal(cyextvOn(true, false), false);
  assert.equal(cyextvOn(true, true), true);
  assert.equal(cyextvOn(false, true), false);
  assert.equal(cyextvOn(true), CYEXTV_READY);
});

test('V112-23 the picker hides its cyex.tv tab until ready', () => {
  assert.ok(!pickerTabs(false).includes('cyextv'));
  assert.deepEqual(pickerTabs(true), ['twitch', 'seventv', 'bttv', 'ffz', 'cyextv']);
  assert.equal(pickerTabs().includes('cyextv'), CYEXTV_READY);
});

test('V112-23 the cyex.tv settings group shows as coming soon until ready', () => {
  const g = GROUPS.find((x) => x.key === 'cyextv');
  assert.equal(!!g?.soon, !CYEXTV_READY);
  assert.ok(GROUPS.filter((x) => x.key !== 'cyextv').every((x) => !x.soon));
});
