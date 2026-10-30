import assert from 'node:assert/strict';
import { test } from 'node:test';

import { nextText, segmentTime, when } from './schedule.ts';

const at = (y: number, mo: number, d: number, h: number, mi: number) => new Date(y, mo - 1, d, h, mi).toISOString();
const next = (start: string) => ({ id: '1', start, title: '', category: null });

test('V108-N03 when reads local time as yyyy/mm/dd HH:mm', () => {
  assert.equal(when(at(2026, 10, 30, 20, 5)), '2026/10/30 20:05');
  assert.equal(when('soon'), '');
});

test('V108-N03 next shows for a stream that has not started', () => {
  const now = new Date(2026, 9, 30, 12, 0).getTime();
  assert.equal(nextText(next(at(2026, 10, 30, 20, 0)), now), 'next: 2026/10/30 20:00');
  assert.equal(nextText(next(at(2026, 10, 30, 11, 0)), now), '');
  assert.equal(nextText(undefined, now), '');
  assert.equal(nextText(next('nope'), now), '');
});

test('V108-N03 a segment shows its end, with the date when it is on another day', () => {
  assert.equal(segmentTime({ start: at(2026, 10, 30, 20, 0), end: at(2026, 10, 30, 22, 30) }), '2026/10/30 20:00–22:30');
  assert.equal(segmentTime({ start: at(2026, 10, 30, 23, 0), end: at(2026, 10, 31, 1, 0) }), '2026/10/30 23:00–2026/10/31 01:00');
  assert.equal(segmentTime({ start: at(2026, 10, 30, 20, 0), end: null }), '2026/10/30 20:00');
  assert.equal(segmentTime({ start: at(2026, 10, 30, 20, 0), end: at(2026, 10, 30, 19, 0) }), '2026/10/30 20:00');
});
