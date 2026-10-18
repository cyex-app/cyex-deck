import assert from 'node:assert/strict';
import { test } from 'node:test';

import { delayVideo, MAX_DELAY, measured, settle, shifted } from './avsync.ts';

test('the delay is both latencies plus the lookahead', () => {
  assert.equal(measured(0.01, 0.04), 0.056);
  assert.equal(measured(0, 0), 0);
  assert.equal(measured(NaN, 0.02), 0.026);
  assert.equal(measured(1, 1), MAX_DELAY);
});

test('jitter keeps the delay, a real change moves it', () => {
  assert.equal(settle(0.056, 0.06), 0.056);
  assert.equal(settle(0.056, 0.08), 0.08);
  assert.equal(settle(0, 0.056), 0.056);
});

test('only video moves later', () => {
  assert.equal(shifted(-10, 'video', 0.05), -10 + 0.05);
  assert.equal(shifted(-10, 'audio', 0.05), -10);
  assert.equal(shifted(undefined, 'video', 0.05), undefined);
  assert.equal(shifted(-10, 'video', 0), -10);
});

function fake() {
  const seen: { event: string; data: { type: string; offset?: number } }[] = [];
  const hls = {
    trigger(event: string, data: unknown) {
      seen.push({ event, data: { ...(data as { type: string; offset?: number }) } });
      return true;
    },
  };
  return { hls, seen };
}

const APPEND = 'hlsBufferAppending';
const frag = { sn: 3 };

test('video appends get the delay before hls.js reads them', () => {
  const { hls, seen } = fake();
  let delay = 0.056;
  let refills = 0;
  delayVideo(hls, APPEND, () => delay, () => (refills++, true));
  hls.trigger(APPEND, { type: 'video', offset: -100, frag });
  hls.trigger(APPEND, { type: 'audio', offset: -100, frag });
  hls.trigger('hlsOther', { type: 'video', offset: -100 });
  assert.equal(seen[0].data.offset, -100 + 0.056);
  assert.equal(seen[1].data.offset, -100);
  assert.equal(seen[2].data.offset, -100);
  // Boosted from the start: nothing to refetch.
  assert.equal(refills, 0);
  delay = 0.058;
  hls.trigger(APPEND, { type: 'video', offset: -100, frag });
  assert.equal(seen[3].data.offset, -100 + 0.056);
});

test('a delay that starts mid-stream reloads the buffer once', async () => {
  const { hls, seen } = fake();
  let delay = 0;
  let refills = 0;
  delayVideo(hls, APPEND, () => delay, () => (refills++, true));
  hls.trigger(APPEND, { type: 'video', offset: -100, frag });
  delay = 0.05;
  hls.trigger(APPEND, { type: 'video', offset: -100, frag });
  hls.trigger(APPEND, { type: 'video', offset: -100, frag });
  await new Promise((r) => setTimeout(r, 5));
  assert.equal(seen[0].data.offset, -100);
  assert.equal(seen[1].data.offset, -100 + 0.05);
  assert.equal(refills, 1);
});

test('init segments are left alone', () => {
  const { hls, seen } = fake();
  delayVideo(hls, APPEND, () => 0.05, () => true);
  hls.trigger(APPEND, { type: 'video', frag: { sn: 'initSegment' } });
  assert.equal(seen[0].data.offset, undefined);
});

test('check: a new delay with video buffered resyncs at once, not before any append', () => {
  const { hls, seen } = fake();
  let delay = 0;
  let resyncs = 0;
  const check = delayVideo(hls, APPEND, () => delay, () => (resyncs++, true));
  delay = 0.05;
  check();
  // Nothing buffered yet: the first append just carries it.
  assert.equal(resyncs, 0);
  hls.trigger(APPEND, { type: 'video', offset: -100, frag });
  assert.equal(seen[0].data.offset, -100 + 0.05);
  delay = 0.08;
  check();
  check();
  assert.equal(resyncs, 1);
  hls.trigger(APPEND, { type: 'video', offset: -100, frag });
  assert.equal(seen[1].data.offset, -100 + 0.08);
  assert.equal(resyncs, 1);
});

test('check: a resync that cannot run leaves it to the next append', async () => {
  const { hls, seen } = fake();
  let delay = 0;
  let can = false;
  let resyncs = 0;
  const check = delayVideo(hls, APPEND, () => delay, () => (resyncs++, can));
  hls.trigger(APPEND, { type: 'video', offset: -100, frag });
  delay = 0.05;
  check();
  assert.equal(resyncs, 1);
  can = true;
  hls.trigger(APPEND, { type: 'video', offset: -100, frag });
  await new Promise((r) => setTimeout(r, 5));
  assert.equal(seen[1].data.offset, -100 + 0.05);
  assert.equal(resyncs, 2);
});

test('an unread delay keeps the one in use and asks again', () => {
  const { hls, seen } = fake();
  let delay: number | null = null;
  let resyncs = 0;
  const check = delayVideo(hls, APPEND, () => delay, () => (resyncs++, true));
  hls.trigger(APPEND, { type: 'video', offset: -100, frag });
  assert.equal(check(), false);
  assert.equal(seen[0].data.offset, -100);
  delay = 0.056;
  assert.equal(check(), true);
  assert.equal(resyncs, 1);
});
