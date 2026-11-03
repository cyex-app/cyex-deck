import assert from 'node:assert/strict';
import { test } from 'node:test';

import { enqueue, gradient, isGoldenKappa, levelSpan, LEVEL_MS, lookOf, msLeft, PALETTE, pctText, sparks, timerFraction, unlockLines } from './hypeLook.ts';

test('V108-C35b a Golden Kappa train is gold, every other one pastel', () => {
  assert.equal(lookOf({ goldenKappa: true }), 'gold');
  assert.equal(lookOf({ goldenKappa: false }), 'pastel');
  assert.equal(lookOf(null), 'pastel');
  assert.ok(gradient('gold').includes(PALETTE.gold[0]) && gradient('gold').includes(PALETTE.gold[2]));
  assert.ok(gradient('pastel').startsWith('linear-gradient(90deg, #ffbfa3'));
  assert.notEqual(gradient('gold'), gradient('pastel'));
});

test('V108-C35b percent text is clamped and whole', () => {
  assert.equal(pctText(45.9), '45 %');
  assert.equal(pctText(140), '100 %');
  assert.equal(pctText(-3), '0 %');
});

test('V108-C35b the level timer counts down from 5 minutes, or more when given', () => {
  assert.equal(levelSpan(120_000), LEVEL_MS);
  assert.equal(levelSpan(420_000), 420_000);
  assert.equal(timerFraction(150_000, LEVEL_MS), 0.5);
  assert.equal(timerFraction(900_000, LEVEL_MS), 1);
  assert.equal(timerFraction(0, LEVEL_MS), 0);
  assert.equal(timerFraction(10, 0), 0);
  const end = Date.parse('2026-11-03T10:05:00Z');
  assert.equal(msLeft('2026-11-03T10:05:00Z', end - 1500), 1500);
  assert.equal(msLeft('2026-11-03T10:05:00Z', end + 1), 0);
  assert.equal(msLeft(null, 0), 0);
  assert.equal(msLeft('nope', 0), 0);
});

test('V108-C35b sparkles: fixed paths, gold ones for a golden train', () => {
  for (const kind of ['burst', 'shower', 'ring'] as const) {
    const a = sparks(kind, 'pastel');
    assert.ok(a.length >= 6);
    assert.deepEqual(a, sparks(kind, 'pastel'));
    assert.ok(a.every((s) => s.delay >= 0 && s.delay <= 400 && s.size > 0));
    assert.notDeepEqual(a.map((s) => s.color), sparks(kind, 'gold').map((s) => s.color));
  }
  // The end shower stays on the bar.
  assert.ok(sparks('shower', 'gold').every((s) => s.x > 0 && s.x < 100));
});

test('V108-C35b the unlock card names the Golden Kappa and its day', () => {
  assert.ok(isGoldenKappa('80393') && !isGoldenKappa('25'));
  assert.deepEqual(unlockLines({ id: '80393', name: 'Kappa' }), { title: 'Golden Kappa unlocked', sub: 'Yours for 24 hours' });
  assert.deepEqual(unlockLines({ id: '1902', name: 'Keepo' }), { title: 'Hype train emote unlocked', sub: 'Keepo · yours to keep' });
});

test('V108-C35b unlocked emotes show one at a time, no repeats, Golden Kappa first', () => {
  const e = (id: string) => ({ id });
  assert.deepEqual(enqueue(null, [], [e('1'), e('2'), e('1')]), [e('1'), e('2')]);
  assert.deepEqual(enqueue(e('1'), [e('2')], [e('1'), e('2'), e('3')]), [e('2'), e('3')]);
  assert.deepEqual(enqueue(null, [e('2')], [e('80393')]).map((x) => x.id), ['80393', '2']);
});
