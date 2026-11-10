import assert from 'node:assert/strict';
import { test } from 'node:test';

import { recordingStopText } from './recordingStop.ts';

test('V110-F92 only a size stop says so', () => {
  assert.equal(recordingStopText({ recording: false, reason: 'size' }), 'Recording stopped at 500 MB.');
  assert.equal(recordingStopText({ recording: false, reason: null }), null);
  assert.equal(recordingStopText({ recording: false }), null);
  assert.equal(recordingStopText({ recording: true, reason: 'size' }), null);
});
