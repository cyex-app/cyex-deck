import assert from 'node:assert/strict';
import { test } from 'node:test';

import { dayStarts } from './dayStarts.ts';

const at = (y: number, mo: number, d: number, h = 12) => new Date(y, mo, d, h).getTime();

test('only lines that leave the day before them start one', () => {
  const now = at(2026, 9, 30, 20);
  const lines = [
    { id: 'a', ts: at(2026, 9, 29, 23) },
    { id: 'b', ts: at(2026, 9, 29, 23) },
    { id: 'c', ts: at(2026, 9, 30, 0) },
    { id: 'd', ts: at(2026, 9, 30, 19) },
  ];
  assert.deepEqual([...dayStarts(lines, now)], ['a', 'c']);
});

test('a chat from today gets no dates', () => {
  const now = at(2026, 9, 30, 20);
  assert.equal(dayStarts([{ id: 'a', ts: at(2026, 9, 30, 1) }, { id: 'b', ts: at(2026, 9, 30, 19) }], now).size, 0);
});

test('midnight belongs to the new day; going back a day starts one too', () => {
  const now = at(2026, 9, 30, 20);
  const lines = [
    { id: 'a', ts: new Date(2026, 9, 30, 0, 0, 0, 0).getTime() },
    { id: 'b', ts: new Date(2026, 9, 29, 23, 59, 59, 999).getTime() },
  ];
  assert.deepEqual([...dayStarts(lines, now)], ['b']);
});
