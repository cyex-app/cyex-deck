import assert from 'node:assert/strict';
import { test } from 'node:test';

import { retryBlockedText } from './retryBlocked.ts';

test('V110-N6 says opened when nothing is blocked any more', () => {
  assert.deepEqual(retryBlockedText({ recovered: ['chat', 'tabs'], blocked: [] }), { ok: true, text: 'Opened again.' });
});

test('V110-N6 names what is still shut, in menu order', () => {
  const r = retryBlockedText({ recovered: [], blocked: ['tabs', 'chat'] });
  assert.equal(r.ok, false);
  assert.equal(r.text, 'Still can’t open Chat, Tabs. Close whatever is using the file and try again.');
});
