import assert from 'node:assert/strict';
import { test } from 'node:test';

import { recordingStopText, recordingWarnText } from './recordingStop.ts';

test('V110-F92 only a size stop says so', () => {
  assert.equal(recordingStopText({ recording: false, reason: 'size' }), 'Stopped at 5 GB.');
  assert.equal(recordingStopText({ recording: false, reason: null }), null);
  assert.equal(recordingStopText({ recording: false }), null);
  assert.equal(recordingStopText({ recording: true, reason: 'size' }), null);
  assert.equal(recordingStopText({ recording: true, reason: 'warn' }), null);
});

test('V110-N1 a write stop says the file can’t be written', () => {
  assert.equal(
    recordingStopText({ recording: false, reason: 'write' }),
    'Recording stopped: the file can’t be written.',
  );
  assert.equal(recordingStopText({ recording: true, reason: 'write' }), null);
});

test('V110-A27 the warning past 1 GB is only for a recording that goes on', () => {
  assert.equal(recordingWarnText({ recording: true, reason: 'warn' }), 'Recording is over 1 GB.');
  assert.equal(recordingWarnText({ recording: false, reason: 'warn' }), null);
  assert.equal(recordingWarnText({ recording: true }), null);
  assert.equal(recordingWarnText({ recording: false, reason: 'size' }), null);
});
