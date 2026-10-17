import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { LogLine } from '../../ipc/bindings/LogLine';
import { CHIPS, visible } from './minerLog.ts';

const line = (id: number, kind: LogLine['kind']): LogLine => ({ id, t: id, kind, channel: null, text: '' });
const lines = [line(1, 'points'), line(2, 'bet'), line(3, 'error'), line(4, 'bet')];

test('visible is newest first', () => {
  assert.deepEqual(visible(lines, new Set()).map((l) => l.id), [4, 3, 2, 1]);
});

test('visible drops the kinds that are off', () => {
  assert.deepEqual(visible(lines, new Set(['bet'])).map((l) => l.id), [3, 1]);
});

test('every kind has a chip', () => {
  assert.deepEqual(CHIPS.map((c) => c.key), ['points', 'claim', 'raid', 'bet', 'drop', 'channel', 'error']);
});
