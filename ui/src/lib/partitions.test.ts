import assert from 'node:assert/strict';
import { test } from 'node:test';

import { importedText } from './partitions.ts';

test('V110-F92 the import toast warns only when automations came in', () => {
  assert.ok(importedText(['general', 'commands']).endsWith(' Automations are off until you switch them on.'));
  assert.ok(importedText(['general']).startsWith('Imported '));
  assert.ok(!importedText(['chat']).includes('Automations'));
});
