// V107-R9 Bell dates in the app's format, local time. Berlin: UTC+1 in November, UTC+2 in July.
process.env.TZ = 'Europe/Berlin';

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { localDates } from './dates.ts';

test('month-name dates with am/pm and UTC', () => {
  assert.equal(
    localDates('Your sub ends on November 10, 2026 7:59:00 am UTC.'),
    'Your sub ends on 2026/11/10 08:59.',
  );
  assert.equal(localDates('Starts Nov 10, 2026 at 7:59 PM UTC'), 'Starts 2026/11/10 20:59');
  assert.equal(localDates('Ends July 1st, 2026, 11:30 p.m. UTC!'), 'Ends 2026/07/02 01:30!');
  assert.equal(localDates('until Dec 31, 2026 11:00 pm GMT'), 'until 2027/01/01 00:00');
  assert.equal(localDates('12:05 am: January 5, 2026 12:05 AM UTC'), '12:05 am: 2026/01/05 01:05');
});

test('day-first and date-only', () => {
  assert.equal(localDates('on 10 November 2026, 19:59 UTC'), 'on 2026/11/10 20:59');
  assert.equal(localDates('Renews on November 10, 2026.'), 'Renews on 2026/11/10.');
  assert.equal(localDates('**xqc** gifted a sub on Sept 3, 2026'), '**xqc** gifted a sub on 2026/09/03');
});

test('ISO dates', () => {
  assert.equal(localDates('at 2026-11-10T07:59:00Z'), 'at 2026/11/10 08:59');
  assert.equal(localDates('at 2026-11-10T07:59:00.123Z.'), 'at 2026/11/10 08:59.');
  assert.equal(localDates('at 2026-07-01 10:00 UTC'), 'at 2026/07/01 12:00');
  assert.equal(localDates('at 2026-11-10T07:59:00-05:00'), 'at 2026/11/10 13:59');
  assert.equal(localDates('on 2026-11-10'), 'on 2026/11/10');
});

test('text without dates stays as is', () => {
  for (const t of [
    '**xqc** went live: Just Chatting',
    'Watch 2026 hours of streams',
    'May the best team win, 10 to 3',
    'Score 13:45 in round 2',
    'Nov 99, 2026 is not a day',
    '',
  ]) {
    assert.equal(localDates(t), t);
  }
});
