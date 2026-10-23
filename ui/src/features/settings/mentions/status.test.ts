import assert from 'node:assert/strict';
import { test } from 'node:test';

import { fileName, goesNowhere, statusText } from './status.ts';

test('the firehose status reads short (V104-02)', () => {
  assert.equal(statusText({ state: 'connected' }, true), 'Connected');
  assert.equal(statusText({ state: 'offline', retrySecs: 8 }, true), 'Offline. Trying again in 8s.');
  assert.equal(statusText({ state: 'off' }, false), 'Add a mention to start.');
  assert.equal(statusText({ state: 'off' }, true), 'Off');
});

test('a sound file shows its name', () => {
  assert.equal(fileName('C:\\Sounds\\ping.wav'), 'ping.wav');
  assert.equal(fileName('ping.wav'), 'ping.wav');
});

test('a global row that goes nowhere says so (V107-H16)', () => {
  const r = { enabled: true, pattern: 'deck', show: false, route: '' };
  assert.equal(goesNowhere(r), 'Goes nowhere: pick a feed or keep Mentions on.');
  assert.equal(goesNowhere({ ...r, route: ' ' }), 'Goes nowhere: pick a feed or keep Mentions on.');
  assert.equal(goesNowhere({ ...r, show: true }), null);
  assert.equal(goesNowhere({ ...r, route: 'Deck' }), null);
  assert.equal(goesNowhere({ ...r, enabled: false }), null);
  assert.equal(goesNowhere({ ...r, pattern: '' }), null);
});
