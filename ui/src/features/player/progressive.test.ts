import assert from 'node:assert/strict';
import { test } from 'node:test';

import { EMPTY_MOOF, FMP4_CUT, Fmp4Gate, timescales } from './progressive.ts';

function box(type: string, ...parts: Uint8Array[]): Uint8Array {
  const len = 8 + parts.reduce((n, p) => n + p.length, 0);
  const out = new Uint8Array(len);
  new DataView(out.buffer).setUint32(0, len);
  for (let i = 0; i < 4; i++) out[4 + i] = type.charCodeAt(i);
  let at = 8;
  for (const p of parts) {
    out.set(p, at);
    at += p.length;
  }
  return out;
}
const words = (...n: number[]) => {
  const out = new Uint8Array(n.length * 4);
  n.forEach((x, i) => new DataView(out.buffer).setUint32(i * 4, x));
  return out;
};
const cat = (...a: Uint8Array[]) => box('xxxx', ...a).slice(8);

/** An init segment: track `id` at `scale` per second (version 0 tkhd/mdhd). */
const init = (id: number, scale: number) =>
  box('moov', box('trak', box('tkhd', words(0, 0, 0, id)), box('mdia', box('mdhd', words(0, 0, 0, scale)))));
/** One fragment: moof (tfhd + v1 tfdt at `t` s) and an mdat of `n` bytes. */
function frag(t: number, n = 100, id = 1, scale = 90000): Uint8Array {
  const tfdt = new Uint8Array(12);
  tfdt[0] = 1;
  new DataView(tfdt.buffer).setBigUint64(4, BigInt(Math.round(t * scale)));
  return cat(box('moof', box('mfhd', words(0, 1)), box('traf', box('tfhd', words(0, id)), box('tfdt', tfdt))), box('mdat', new Uint8Array(n)));
}

test('V108-LL3 timescales read from the init segment', () => {
  assert.deepEqual([...timescales(init(1, 90000))], [[1, 90000]]);
  assert.equal(timescales(new Uint8Array(4)).size, 0);
});

test('V108-LL3 an fMP4 segment streams until the cut, the rest comes whole at the end', () => {
  const g = new Fmp4Gate(timescales(init(1, 90000)));
  const base = 9931.749; // a stream's clock, far from 0
  const fs = [0, 0.3, 0.6, 0.85, 1.2, 1.5, 1.8].map((s) => frag(base + s));
  const all = cat(...fs);
  const sent: Uint8Array[] = [];
  for (const f of fs) {
    const out = g.push(f);
    if (out) sent.push(out);
  }
  // Each piece goes once the next moof is in: 0, 0.3 and 0.6; 0.85 is held with the rest
  // (the 1.2 moof is past the cut), so the held part starts before it.
  assert.deepEqual(
    sent.map((s) => s.length),
    [fs[0].length, fs[1].length, fs[2].length],
  );
  const rest = g.end();
  assert.equal(rest.length, fs.slice(3).reduce((n, f) => n + f.length, 0) + EMPTY_MOOF.length);
  assert.deepEqual(cat(...sent, rest), cat(all, EMPTY_MOOF), 'every byte once, in order, the empty moof last');
  assert.ok(0.85 < FMP4_CUT);
});

test('V108-LL3 pieces split anywhere, boxes cut across chunks', () => {
  const g = new Fmp4Gate(timescales(init(1, 90000)));
  const all = cat(...[0, 0.4, 0.8, 1.2].map((s) => frag(100 + s, 5000)));
  const sent: Uint8Array[] = [];
  for (let at = 0; at < all.length; at += 777) {
    const out = g.push(all.slice(at, at + 777));
    if (out) sent.push(out);
  }
  assert.ok(sent.length >= 1);
  assert.deepEqual(cat(...sent, g.end()), cat(all, EMPTY_MOOF));
});

test('V108-LL3 an unknown track holds the whole segment', () => {
  const g = new Fmp4Gate(timescales(init(2, 90000)));
  const all = cat(frag(0), frag(0.3), frag(0.6));
  assert.equal(g.push(all), null);
  assert.deepEqual(g.end(), cat(all, EMPTY_MOOF));
});
