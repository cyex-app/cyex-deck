import assert from 'node:assert/strict';
import { type TestContext, test } from 'node:test';

import { AWAY_MS, Away } from './away.ts';

function setup(t: TestContext) {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  let drops = 0;
  return { away: new Away(() => drops++), drops: () => drops };
}

test('back within the minute nothing changes (V103-03)', (t) => {
  const { away, drops } = setup(t);
  away.hide();
  t.mock.timers.tick(AWAY_MS - 1);
  assert.equal(away.show(), false, 'nothing to restore');
  t.mock.timers.tick(AWAY_MS);
  assert.equal(drops(), 0, 'the timer was cancelled');
  assert.equal(away.low, false);
});

test('a minute away drops once, back restores', (t) => {
  const { away, drops } = setup(t);
  away.hide();
  t.mock.timers.tick(AWAY_MS / 2);
  away.hide();
  t.mock.timers.tick(AWAY_MS / 2);
  assert.equal(drops(), 1, 'a second report keeps the first start');
  assert.equal(away.low, true);
  away.hide();
  t.mock.timers.tick(AWAY_MS * 2);
  assert.equal(drops(), 1, 'already low: no new timer');
  assert.equal(away.show(), true, 'its quality comes back');
  assert.equal(away.low, false);
  assert.equal(away.show(), false, 'only once');
});

test('away again after coming back starts a fresh minute', (t) => {
  const { away, drops } = setup(t);
  away.hide();
  t.mock.timers.tick(AWAY_MS);
  away.show();
  away.hide();
  t.mock.timers.tick(AWAY_MS - 1);
  assert.equal(drops(), 1);
  t.mock.timers.tick(1);
  assert.equal(drops(), 2);
});

test('a closed player never drops', (t) => {
  const { away, drops } = setup(t);
  away.hide();
  away.stop();
  t.mock.timers.tick(AWAY_MS * 2);
  assert.equal(drops(), 0);
});
