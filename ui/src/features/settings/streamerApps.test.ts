import assert from 'node:assert/strict';
import { test } from 'node:test';

import { validAppName } from './streamerApps.ts';

test('V110-F101 program names', () => {
  for (const ok of ['obs64.exe', 'obs64', ' Streamlabs OBS ', 'x'.repeat(80)]) assert.equal(validAppName(ok), true, ok);
  for (const bad of ['', '   ', 'a<b', 'a>b', 'a:b', 'a"b', 'a/b', 'a\b', 'a|b', 'a?b', 'a*b', 'a\u0007b']) {
    assert.equal(validAppName(bad), false, JSON.stringify(bad));
  }
});
