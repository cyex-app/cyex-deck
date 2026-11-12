import assert from 'node:assert/strict';
import { test } from 'node:test';

import { endPhase, pollEnded, predictionEnded } from './endedFold.ts';

const T = 1_000_000;

test('V110-N6 a live item never folds or goes', () => {
  assert.equal(endPhase(null, T + 10 * 60_000, false), 'open');
});

test('V110-N6 ended: open at +29 s, folded at +31 s, gone at +61 s', () => {
  assert.equal(endPhase(T, T + 29_000, false), 'open');
  assert.equal(endPhase(T, T + 31_000, false), 'folded');
  assert.equal(endPhase(T, T + 61_000, false), 'gone');
});

test('V110-N6 unfolded by hand: stays open until the minute is up', () => {
  assert.equal(endPhase(T, T + 31_000, true), 'open');
  assert.equal(endPhase(T, T + 59_000, true), 'open');
  assert.equal(endPhase(T, T + 61_000, true), 'gone');
});

test('V110-N6 which statuses count as ended', () => {
  for (const s of ['COMPLETED', 'TERMINATED', 'ARCHIVED']) assert.equal(pollEnded(s), true);
  assert.equal(pollEnded('ACTIVE'), false);
  for (const s of ['RESOLVED', 'CANCELED']) assert.equal(predictionEnded(s), true);
  for (const s of ['ACTIVE', 'LOCKED', 'RESOLVE_PENDING', 'CANCEL_PENDING']) assert.equal(predictionEnded(s), false);
});

test('V110-N6 the banner folds and closes through endPhase', async () => {
  const { readFileSync } = await import('node:fs');
  const src = readFileSync(new URL('./VotesBanner.svelte', import.meta.url), 'utf8');
  assert.match(src, /endPhase\(/);
  assert.match(src, /eventStrip\.close\(channel, pollKey\(poll\.id\)\)/);
  assert.match(src, /eventStrip\.close\(channel, predictionKey\(pred\.id\)\)/);
});
