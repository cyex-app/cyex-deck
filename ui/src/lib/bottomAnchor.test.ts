import assert from 'node:assert/strict';
import { test } from 'node:test';

import { anchoredTop, gapBelow, isPinned } from './bottomAnchor.ts';

test('gap and pinned', () => {
  assert.equal(gapBelow({ scrollHeight: 1000, scrollTop: 600, clientHeight: 400 }), 0);
  assert.equal(gapBelow({ scrollHeight: 1000, scrollTop: 300, clientHeight: 400 }), 300);
  assert.equal(gapBelow({ scrollHeight: 1000, scrollTop: 700, clientHeight: 400 }), 0);
  assert.ok(isPinned({ scrollHeight: 1000, scrollTop: 590, clientHeight: 400 }));
  assert.ok(!isPinned({ scrollHeight: 1000, scrollTop: 500, clientHeight: 400 }));
});

test('pinned stays at the end when the viewport shrinks or grows', () => {
  assert.equal(anchoredTop({ scrollHeight: 1000, clientHeight: 300 }, 0, true), 700);
  assert.equal(anchoredTop({ scrollHeight: 1000, clientHeight: 500 }, 0, true), 500);
});

test('scrolled up keeps its bottom gap when the viewport shrinks', () => {
  // gap 300 before (client 400, top 300); viewport shrinks to 300: bottom line stays put.
  assert.equal(anchoredTop({ scrollHeight: 1000, clientHeight: 300 }, 300, false), 400);
});

test('scrolled up keeps its bottom gap when the viewport grows, never negative', () => {
  assert.equal(anchoredTop({ scrollHeight: 1000, clientHeight: 500 }, 300, false), 200);
  assert.equal(anchoredTop({ scrollHeight: 400, clientHeight: 500 }, 300, false), 0);
});
