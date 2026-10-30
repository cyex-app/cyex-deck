import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Moment } from '../../ipc/bindings/Moment';
import { awayText, catchOf, isAway, isQuiet, lineAt, markerFor, summaryText } from './catchUp.ts';

const line = (id: string, ts: number, extra: Record<string, unknown> = {}) =>
  ({ id, ts, flags: [] as never[], ...extra }) as Parameters<typeof catchOf>[0][number];
const moment = (id: number, atMs: number): Moment => ({ id, atMs, streamSecs: null, kind: 'raid', label: '', score: 0 });

test('away needs the full time (V108-N18)', () => {
  assert.ok(!isAway(0, 4 * 60_000, 5));
  assert.ok(isAway(0, 5 * 60_000, 5));
  assert.ok(isAway(0, 60_000, 0));
});

test('the marker is the first line after you left (V108-N18)', () => {
  const lines = [line('a', 10), line('b', 20), line('c', 30), line('d', 40)];
  assert.deepEqual(markerFor(lines, 20), { id: 'c', at: 2, trimmed: false });
  assert.deepEqual(markerFor(lines, 40), { id: null, at: -1, trimmed: false });
  assert.deepEqual(markerFor([], 5), { id: null, at: -1, trimmed: false });
});

test('lines the budget trimmed put the marker on the oldest one (V108-N18)', () => {
  const lines = [line('x', 100), line('y', 110)];
  assert.deepEqual(markerFor(lines, 50), { id: 'x', at: 0, trimmed: true });
});

test('counts mentions, highlights, events and moments after the marker (V108-N18)', () => {
  const lines = [
    line('a', 10, { flags: ['mention'] }),
    line('b', 30, { flags: ['mention'] }),
    line('c', 31, { highlight: { kind: 'rule' } }),
    line('d', 32, { notice: { kind: 'sub' }, highlight: { kind: 'sub' } }),
    line('e', 33, { notice: { kind: 'raid' } }),
    line('f', 34, { flags: ['own'] }),
    line('g', 35, { highlight: { kind: 'mention' } }),
  ];
  const marker = markerFor(lines, 20);
  const c = catchOf(lines, marker, [moment(1, 15), moment(2, 32), moment(3, 34)], 20);
  assert.deepEqual(c.summary, { total: 5, mentions: 2, highlights: 1, subs: 1, raids: 1, moments: 2 });
  assert.deepEqual(c.mentionIds, ['g', 'b']);
  assert.deepEqual(
    c.moments.map((m) => m.id),
    [3, 2],
  );
  assert.equal(summaryText(c.summary), '2 mentions · 1 highlight · 1 sub event · 1 raid · 2 moments');
});

test('a quiet stretch has no card (V108-N18)', () => {
  const lines = [line('a', 30), line('b', 31)];
  const c = catchOf(lines, markerFor(lines, 20), [], 20);
  assert.equal(c.summary.total, 2);
  assert.ok(isQuiet(c.summary));
  assert.equal(summaryText(c.summary), '');
});

test('moments with no line yet and no marker still count (V108-N18)', () => {
  const c = catchOf([], markerFor([], 20), [moment(1, 25)], 20);
  assert.equal(c.summary.moments, 1);
  assert.ok(!isQuiet(c.summary));
});

test('lineAt finds the first loaded line from a time on (V108-N18)', () => {
  const lines = [line('a', 10), line('b', 20), line('c', 30)];
  assert.equal(lineAt(lines, 15), 'b');
  assert.equal(lineAt(lines, 20), 'b');
  assert.equal(lineAt(lines, 31), null);
  assert.equal(lineAt(lines, 0), 'a');
});

test('away text reads short (V108-N18)', () => {
  assert.equal(awayText(30_000), '1 min');
  assert.equal(awayText(7 * 60_000), '7 min');
  assert.equal(awayText(65 * 60_000), '1 h 5 min');
  assert.equal(awayText(120 * 60_000), '2 h');
});
