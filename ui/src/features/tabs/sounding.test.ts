import assert from 'node:assert/strict';
import { test } from 'node:test';

import { Sounding } from './sounding.ts';

type Fake = EventTarget & { paused: boolean; ended: boolean };

function media(): Fake {
  return Object.assign(new EventTarget(), { paused: true, ended: false });
}

function play(m: Fake): void {
  m.paused = false;
  m.dispatchEvent(new Event('play'));
}

const one = new Set(['t1']);

test('V106-25 a video that plays would be heard; paused or ended it wouldn’t', () => {
  const s = new Sounding();
  const m = media();
  s.watch('t1', m, () => true);
  assert.equal(s.wouldPlay(one), false);
  play(m);
  assert.equal(s.wouldPlay(one), true);
  m.paused = true;
  m.dispatchEvent(new Event('pause'));
  assert.equal(s.wouldPlay(one), false);
  play(m);
  m.ended = true;
  m.dispatchEvent(new Event('ended'));
  assert.equal(s.wouldPlay(one), false);
});

test('V106-25 a video muted in its player counts as silent', () => {
  const s = new Sounding();
  const m = media();
  s.watch('t1', m, () => false);
  play(m);
  assert.equal(s.wouldPlay(one), false);
});

test('V106-25 two videos on one tile: one pauses, the other is still heard', () => {
  const s = new Sounding();
  const a = media();
  const b = media();
  s.watch('t1', a, () => true);
  s.watch('t1', b, () => true);
  play(a);
  play(b);
  a.paused = true;
  a.dispatchEvent(new Event('pause'));
  assert.equal(s.wouldPlay(one), true);
});

test('V106-25 another tile’s video doesn’t count', () => {
  const s = new Sounding();
  const m = media();
  s.watch('t2', m, () => true);
  play(m);
  assert.equal(s.wouldPlay(one), false);
});

test('V106-25 a stopped watch no longer counts', () => {
  const s = new Sounding();
  const m = media();
  const stop = s.watch('t1', m, () => true);
  play(m);
  stop();
  assert.equal(s.wouldPlay(one), false);
  play(m);
  assert.equal(s.wouldPlay(one), false);
});

test('V106-25 play then playing writes once', () => {
  let writes = 0;
  class Counting<K, V> extends Map<K, V> {
    override set(k: K, v: V): this {
      writes += 1;
      return super.set(k, v);
    }
  }
  const s = new Sounding(new Counting());
  const m = media();
  s.watch('t1', m, () => true);
  play(m);
  m.dispatchEvent(new Event('playing'));
  assert.equal(writes, 2);
  assert.equal(s.wouldPlay(one), true);
});
