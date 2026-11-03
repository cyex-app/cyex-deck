import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { HypeTrain } from '../../ipc/bindings/HypeTrain.ts';
import { burstOf, closing, endedText, formatLeft, givenLine, mergeConductor, mergeProgress, progressPct, secondsLeft, topLine, topParts, trainOver } from './hypeTrain.ts';

const train = (o: Partial<HypeTrain> = {}): HypeTrain => ({
  id: 't1',
  level: 2,
  progress: 300,
  goal: 1200,
  total: 1500,
  startedAt: 'a',
  expiresAt: '2026-10-29T18:05:00Z',
  endedAt: null,
  goldenKappa: false,
  treasure: false,
  conductors: [{ source: 'BITS', login: 'a', name: 'A', total: 5 }],
  subs: 0,
  bits: 0,
  ...o,
});

test('hype train progress is clamped (V108-N04)', () => {
  assert.equal(progressPct(300, 1200), 25);
  assert.equal(progressPct(5000, 1200), 100);
  assert.equal(progressPct(10, 0), 0);
  assert.equal(progressPct(-1, 10), 0);
});

test('hype train time left counts down to zero', () => {
  const end = Date.parse('2026-10-29T18:05:00Z');
  assert.equal(secondsLeft('2026-10-29T18:05:00Z', end - 247_000), 247);
  assert.equal(secondsLeft('2026-10-29T18:05:00Z', end + 5000), 0);
  assert.equal(secondsLeft(null, 0), 0);
  assert.equal(secondsLeft('nope', 0), 0);
  assert.equal(formatLeft(247), '4:07');
  assert.equal(formatLeft(0), '0:00');
  assert.equal(formatLeft(-3), '0:00');
});

test('hype train progress keeps conductors and start time', () => {
  const m = mergeProgress(
    train(),
    train({ id: '', level: 3, progress: 10, goal: 1800, total: 1510, startedAt: null, conductors: [] }),
  );
  assert.equal(m.level, 3);
  assert.equal(m.progress, 10);
  assert.equal(m.startedAt, 'a');
  assert.equal(m.conductors.length, 1);
});

test('a different hype train replaces the held one', () => {
  assert.equal(mergeProgress(train(), train({ id: 't2', conductors: [] })).id, 't2');
  assert.equal(mergeProgress(null, train()).id, 't1');
});

test('hype train conductors replace by source and the top one is named', () => {
  const list = mergeConductor(train().conductors, { source: 'BITS', login: 'b', name: 'B', total: 900 });
  assert.equal(list.length, 1);
  assert.equal(topLine(list, (n) => `#${n}`), 'B #900 bits');
  assert.equal(topLine([]), null);
});

test('hype train closing note', () => {
  assert.equal(endedText(4), 'Level 3 reached');
  assert.equal(endedText(1), 'Hype train ended');
});

test('an empty read ends the train only when it ran out or the topic went quiet (V108-N04)', () => {
  const now = Date.parse('2026-10-29T12:00:00Z');
  const later = '2026-10-29T12:05:00Z';
  assert.equal(trainOver(later, now, now - 5_000), false);
  assert.equal(trainOver(later, now, now - 120_000), true);
  assert.equal(trainOver('2026-10-29T11:59:00Z', now, now - 1_000), true);
  assert.equal(trainOver(null, now, now), true);
});

test('V108-C35b progress is a percentage of the level, 100 only once met', () => {
  assert.equal(progressPct(1999, 2000), 99);
  assert.equal(progressPct(2000, 2000), 100);
  assert.equal(progressPct(1, 2000), 0);
});

test('progress messages keep the subs and bits a read found', () => {
  const m = mergeProgress(train({ subs: 4, bits: 300 }), train({ progress: 20, conductors: [] }));
  assert.equal(m.subs, 4);
  assert.equal(m.bits, 300);
});

test('the closing stats take the last read of the same train (V108-N04)', () => {
  const held = train({ subs: 4, bits: 300, level: 3 });
  const last = train({ subs: 13, bits: 1700, level: 4, endedAt: 'z', conductors: [] });
  const c = closing(held, last);
  assert.equal(c.level, 4);
  assert.equal(c.subs, 13);
  assert.equal(c.bits, 1700);
  assert.equal(c.endedAt, 'z');
  assert.equal(c.conductors.length, 1);
  // A read behind the topic doesn't lower what was seen.
  assert.equal(closing(train({ subs: 9 }), train({ subs: 2 })).subs, 9);
  assert.equal(closing(held, train({ id: 'other', subs: 99 })).subs, 4);
  assert.equal(closing(held, null), held);
});

test('subs and bits line', () => {
  assert.equal(givenLine(24, 5300, (n) => `#${n}`), '#24 subs · #5300 bits');
  assert.equal(givenLine(1, 1), '1 sub · 1 bit');
  assert.equal(givenLine(0, 100), '100 bits');
  assert.equal(givenLine(0, 0), null);
});

test('a progress message without a time keeps the held one only while it is ahead (V108-N04)', () => {
  const held = train({ expiresAt: '2026-10-29T18:05:00Z' });
  const before = Date.parse('2026-10-29T18:00:00Z');
  const after = Date.parse('2026-10-29T18:06:00Z');
  assert.equal(mergeProgress(held, train({ id: '', expiresAt: null }), before).expiresAt, '2026-10-29T18:05:00Z');
  assert.equal(mergeProgress(held, train({ id: '', expiresAt: null }), after).expiresAt, null);
  assert.equal(mergeProgress(held, train({ id: '', expiresAt: '2026-10-29T18:09:00Z' }), after).expiresAt, '2026-10-29T18:09:00Z');
  assert.equal(mergeProgress(null, train({ expiresAt: null })).expiresAt, null);
});

test('the top giver comes in two parts so a long name can be cut (V108-N04)', () => {
  const long = [{ source: 'BITS' as const, login: 'x', name: 'N'.repeat(60), total: 1200 }];
  assert.deepEqual(topParts(long, String), { who: 'N'.repeat(60), amount: '1200 bits' });
  assert.deepEqual(topParts([{ source: 'SUBS' as const, login: 'a', name: '', total: 0 }]), { who: 'a', amount: '' });
  assert.equal(topParts([]), null);
  // V108-C35h One sub is singular.
  assert.deepEqual(topParts([{ source: 'SUBS' as const, login: 'a', name: 'A', total: 1 }], String), { who: 'A', amount: '1 sub' });
  assert.deepEqual(topParts([{ source: 'SUBS' as const, login: 'a', name: 'A', total: 5 }], String), { who: 'A', amount: '5 subs' });
  assert.equal(topLine(long, String), `${'N'.repeat(60)} 1200 bits`);
});

test('V108-C35 a start or a level up celebrates, the rest does not', () => {
  const held = train({ id: 't1', level: 2 });
  assert.equal(burstOf(null, { kind: 'start', train: train({ level: 1 }) }), 'start');
  // The start of the train already held is no news.
  assert.equal(burstOf(held, { kind: 'start', train: train({ id: 't1' }) }), null);
  assert.equal(burstOf(null, { kind: 'progress', train: train({ level: 1 }) }), 'start');
  // A train first heard of midway isn't a start.
  assert.equal(burstOf(null, { kind: 'progress', train: train({ level: 3 }) }), null);
  assert.equal(burstOf(held, { kind: 'progress', train: train({ id: 't1', level: 3 }) }), 'level');
  assert.equal(burstOf(held, { kind: 'progress', train: train({ id: '', level: 2 }) }), null);
  assert.equal(burstOf(held, { kind: 'progress', train: train({ id: 't2', level: 1 }) }), 'start');
  assert.equal(burstOf(held, { kind: 'end' }), null);
  assert.equal(burstOf(held, { kind: 'reread' }), null);
});

