import assert from 'node:assert/strict';
import { test } from 'node:test';

import { importedText, inMenuOrder, partitionList } from './partitions.ts';

test('V110-F92 the import toast warns only when automations came in', () => {
  assert.ok(importedText(['general', 'commands']).endsWith(' Automations are off until you switch them on.'));
  assert.ok(importedText(['general']).startsWith('Imported '));
  assert.ok(!importedText(['chat']).includes('Automations'));
});

test('V110-A43 the import toast adds a line for each list that was cut', () => {
  const text = importedText(['commands'], ['Kept the first 500 commands.', 'Kept the first 100 automations.']);
  assert.ok(text.endsWith('Kept the first 500 commands. Kept the first 100 automations.'));
  assert.equal(importedText(['general'], []), 'Imported General.');
});

test('N3-5 the import review lists parts in menu order', () => {
  assert.equal(partitionList(['hotkeys', 'general', 'chat']), 'General, Chat, Hotkeys');
  assert.deepEqual(inMenuOrder(['menus', 'general']), ['general', 'menus']);
});
