import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { HypeTrain } from '../../ipc/bindings/HypeTrain.ts';
import { endedText, formatLeft, mergeConductor, mergeProgress, progressPct, secondsLeft, topLine } from './hypeTrain.ts';

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
