import assert from 'node:assert/strict';
import { test } from 'node:test';

import { factText, numeric } from './linkFacts.ts';

test('card facts use the app formats (V108-C25)', () => {
  assert.equal(factText({ t: 'views', n: 1_234_567 }), '1.234.567 views');
  assert.equal(factText({ t: 'viewers', n: 1 }), '1 viewer');
  assert.equal(factText({ t: 'length', secs: 7384 }), '2h 3m');
  assert.equal(factText({ t: 'date', at: '2026-10-01T12:00:00' }), '2026/10/01');
  assert.equal(factText({ t: 'date', at: 'nope' }), '');
  assert.equal(factText({ t: 'clippedBy', name: 'A' }), 'Clipped by A');
  assert.equal(factText({ t: 'likes', n: 1200 }), '1.200 likes');
  assert.equal(factText({ t: 'likes', n: 1 }), '1 like');
  assert.equal(numeric({ t: 'likes', n: 1 }), true);
  assert.equal(numeric({ t: 'game', name: 'Chess' }), false);
});
