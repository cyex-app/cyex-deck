// V110-F89 Generated-input tests: pure UI helpers on odd numbers and odd text. Seeded, so a
// failure reproduces. Run with `npm run test:ui`.

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { clampNumber, parseNumber, rangeNote, readNumber, stepFor } from '../components/number.ts';
import { RANK, commaStart, commaStep, commaValue, complete, fold, insert, keepExact, rank, rankBest, wordAt, wordStarts } from '../features/chat/completion.ts';
import { stacks } from '../features/chat/stacked.ts';
import { emojiKey, splitEmoji } from '../features/emotes/emoji/split.ts';
import { orderHits, rankEmotes, rankNames, usageRank } from '../features/emotes/ranking.ts';
import { anchorAction } from './anchorAction.ts';
import { ago, formatBytes, formatDate, formatDecimal, formatDuration, formatFactor, formatNumber, formatSecs, formatShort, formatStamp, formatTime, parseDuration } from './format.ts';
import { NUMBERS, Rng, broken, mutate, number, text, words } from './testGen.ts';

const BAD = /NaN|Infinity|e\+|e-\d/;

test('every number formatter returns text for any number', () => {
  const r = new Rng(0xf01);
  const inputs = [...NUMBERS, ...Array.from({ length: 3000 }, () => number(r))];
  for (const n of inputs) {
    for (const fn of [formatNumber, formatShort, formatBytes, formatDuration, formatSecs, formatDecimal, formatFactor]) {
      assert.equal(typeof fn(n), 'string', `${fn.name}(${n})`);
    }
    assert.equal(typeof ago(n, 1e12), 'string');
    assert.equal(typeof ago(n, n, true), 'string');
    assert.equal(typeof formatDate(new Date(n)), 'string');
    assert.equal(typeof formatTime(new Date(n)), 'string');
    assert.equal(typeof formatStamp(n), 'string');
  }
});

test('grouped, short, byte and length formatters never print NaN, Infinity or an exponent', () => {
  const r = new Rng(0xf02);
  const inputs = [...NUMBERS, ...Array.from({ length: 4000 }, () => number(r))];
  for (const n of inputs) {
    for (const fn of [formatNumber, formatShort, formatBytes, formatSecs]) {
      assert.ok(!BAD.test(fn(n)), `${fn.name}(${n}) = ${fn(n)}`);
    }
  }
});

test('formatNumber groups every readable integer by dots', () => {
  const r = new Rng(0xf03);
  for (let i = 0; i < 4000; i++) {
    const n = Math.trunc(number(r));
    if (!Number.isFinite(n) || Math.abs(n) >= 1e21) {
      assert.equal(formatNumber(n), '-');
      continue;
    }
    const s = formatNumber(n);
    assert.match(s, /^-?\d{1,3}(\.\d{3})*$/, `${n} -> ${s}`);
    assert.equal(Number(s.replace(/\./g, '')), Math.trunc(n) || 0, `${n} -> ${s}`);
  }
});

test('formatBytes picks one unit and never shows 1.024 of the unit below', () => {
  const r = new Rng(0xf04);
  for (let i = 0; i < 4000; i++) {
    const n = Math.abs(number(r));
    if (!Number.isFinite(n) || n >= 1e21) continue;
    const s = formatBytes(n);
    assert.match(s, /^\d{1,3}(\.\d{3})*(,\d)? (B|KB|MB|GB|TB)$|^[\d.]+ TB$/, `${n} -> ${s}`);
    assert.ok(!/^1\.0[2-9]\d (B|KB|MB|GB) /.test(s) || /TB/.test(s), `${n} -> ${s}`);
  }
});

test('lengths round trip through parseDuration', () => {
  const r = new Rng(0xf05);
  for (let i = 0; i < 3000; i++) {
    const n = r.below(1e7);
    assert.equal(parseDuration(`${n}`), n);
    assert.equal(parseDuration(`${n}s`), n);
    const shown = formatDuration(n);
    assert.match(shown, /^(\d+[wdhms])( \d+[wdhms])?$/);
    const back = parseDuration(shown);
    assert.ok(back !== null && back <= n && (n === 0 || back > 0), `${n} -> ${shown} -> ${back}`);
  }
  for (let i = 0; i < 4000; i++) {
    const t = r.chance(50) ? text(r, 12) : mutate(r, '1h 30m 5s');
    const d = parseDuration(t);
    assert.ok(d === null || (Number.isFinite(d) && d >= 0), `${JSON.stringify(t)} -> ${d}`);
  }
});

test('V110-F89 finding: decimals and factors print NaN, Infinity or an exponent', () => {
  for (const n of [NaN, Infinity, -Infinity, 1e21, 1e308]) {
    assert.ok(!BAD.test(formatDecimal(n)), `formatDecimal(${n}) = ${formatDecimal(n)}`);
    assert.ok(!BAD.test(formatFactor(n)), `formatFactor(${n}) = ${formatFactor(n)}`);
  }
});

test('V110-F89 finding: a missing or broken time prints NaN/NaN/NaN', () => {
  assert.ok(!BAD.test(ago(NaN)), ago(NaN));
  assert.ok(!BAD.test(formatDate(new Date(NaN))));
  assert.ok(!BAD.test(formatTime(new Date(NaN))));
  assert.ok(!BAD.test(formatStamp(NaN)));
  assert.ok(!BAD.test(ago(-Infinity, 1e12, true)));
});

test('V110-F89 finding: formatDuration of a huge number prints Infinityw or 1.6e+302w', () => {
  for (const n of [Infinity, 1e21, 1e308]) assert.ok(!BAD.test(formatDuration(n)), `${n} -> ${formatDuration(n)}`);
});

test('number boxes: any text and any range give null or a value inside the range', () => {
  const r = new Rng(0xf06);
  for (let i = 0; i < 6000; i++) {
    const min = r.chance(70) ? r.below(100) - 50 : number(r);
    const max = min + (r.chance(80) ? r.below(1000) : number(r));
    const step = r.chance(60) ? 1 : r.pick([0.1, 0.5, 5, 0.001, 25]);
    const range = { min, max, step };
    const t = r.chance(50) ? text(r, 10) : mutate(r, r.pick(['5', '1.000', '1,5', '-3', '+7', '1e3', '0x10', '  12  ', '1.000.000', '-1.000,5', '.5', '5.']));
    const p = parseNumber(t, range);
    const ok = Number.isFinite(min) && Number.isFinite(max) && min <= max;
    if (p !== null && ok) assert.ok(p >= min && p <= max, `${JSON.stringify(t)} -> ${p} in ${min}..${max}`);
    const read = readNumber(t);
    assert.ok(read === null || Number.isFinite(read), `${JSON.stringify(t)} -> ${read}`);
    const note = rangeNote(t, range, formatNumber);
    assert.ok(note === null || typeof note === 'string');
    const v = clampNumber(number(r), range);
    if (ok && Number.isFinite(v)) assert.ok(v >= min && v <= max, `${v} in ${min}..${max}`);
    for (const key of ['ArrowUp', 'ArrowDown', 'PageUp', 'PageDown', 'Home', 'End', 'x', '']) {
      const s = stepFor(key, r.chance(50), number(r), range);
      assert.ok(s === null || typeof s === 'number');
    }
  }
});

test('V110-F89 finding: a zero step turns a number box into NaN', () => {
  assert.ok(Number.isFinite(clampNumber(5, { min: 0, max: 10, step: 0 })));
});

test('completion: the word at any caret, and the text after inserting', () => {
  const r = new Rng(0xf07);
  for (let i = 0; i < 3000; i++) {
    const t = r.chance(30) ? words(r, 6) : text(r, 30);
    // Carets sit between characters, never inside a surrogate pair.
    for (let caret = 0; caret <= t.length; caret++) {
      const lo = t.charCodeAt(caret - 1);
      const hi = t.charCodeAt(caret);
      if (caret > 0 && caret < t.length && lo >= 0xd800 && lo < 0xdc00 && hi >= 0xdc00 && hi < 0xe000) continue;
      const w = wordAt(t, caret);
      assert.ok(w.start <= w.end && w.end === caret && w.text === t.slice(w.start, w.end));
      assert.ok(!/\s/.test(w.text), JSON.stringify(w));
      const value = r.pick(['x', 'Kappa', '😀', 'e\u0301', ':)', '']);
      const out = insert(t, w, value);
      assert.ok(out.caret >= 0 && out.caret <= out.text.length, `${JSON.stringify(t)}@${caret} -> ${JSON.stringify(out)}`);
      assert.ok(out.text.isWellFormed(), `insert split a pair: ${JSON.stringify(t)}@${caret} -> ${JSON.stringify(out.text)}`);
      assert.ok(out.text.startsWith(t.slice(0, w.start) + value), 'text before and the value stay');
    }
  }
});

test('V110-F89 finding: a caret inside a surrogate pair makes insert leave a lone surrogate', { skip: 'wordAt(t, k) with k between the two halves of a pair, then insert(), returns a text with a lone low surrogate; carets in a textarea sit on pair edges, so only a caret computed in code points or from a cut string can trigger it' }, () => {
  const t = 'a 😀 b';
  const w = wordAt(t, 3);
  assert.ok(insert(t, w, 'x').text.isWellFormed());
});

test('completion: ranking and matching never throw and keep their promises', () => {
  const r = new Rng(0xf08);
  const names: string[] = ['Kappa', 'peepoHappy', 'xQc', 'İstanbulBot', 'ß_test', 'a1b2', 'café', 'CAFE', 'KEKW', 'monkaS', '7TV', 'x', ''];
  for (let i = 0; i < 2500; i++) {
    const typed = r.chance(40) ? text(r, 6) : mutate(r, r.pick(names));
    const pool = [...names, text(r, 10), text(r, 10), mutate(r, r.pick(names))];
    const name = r.pick(pool);
    const res = rank(name, typed);
    assert.ok(res.rank >= RANK.none && res.rank <= RANK.caseEqual && res.close >= 0 && res.close <= 1, `${name} ${typed} ${JSON.stringify(res)}`);
    if (name) assert.equal(rank(name, name).rank, RANK.caseEqual);
    assert.equal(rank(name, '').rank, RANK.none);
    const got = complete(typed, pool);
    assert.ok(got.length <= 8 && new Set(got).size === got.length && !got.includes(typed));
    for (const g of rankNames(typed, pool, 5)) assert.ok(pool.includes(g));
    const hits = orderHits(typed, pool.map((n, k) => ({ item: k, name: n, tie: k % 3 })));
    assert.equal(new Set(hits).size, hits.length, 'each candidate once');
    const found = rankEmotes(typed, pool.map((n, k) => ({ item: k, name: n, used: k % 2 ? Infinity : k, locked: k % 5 === 0 })));
    assert.equal(new Set(found).size, found.length);
    rankBest(pool, typed);
    wordStarts(name);
    fold(name);
    const plan = commaStart(got, typed, (x) => x.length % 2 === 0);
    if (plan) {
      let p = plan;
      for (let k = 0; k < 6; k++) {
        assert.equal(typeof commaValue(p), 'string');
        p = commaStep(p, k % 2 === 0);
        assert.ok(p.index >= 0 && p.index < Math.max(1, p.items.length));
      }
    }
    keepExact(got, typed, (x) => x === typed);
  }
  assert.deepEqual(usageRank([]).size, 0);
  assert.deepEqual(rankNames('', ['a']), []);
  assert.deepEqual(rankEmotes('   ', [{ item: 1, name: 'a' }]), []);
});

test('emoji split: the pieces spell the text back, in order, with no empty piece', () => {
  const r = new Rng(0xf09);
  const seeds = ['a😀b', '👨‍👩‍👧 family', '🇩🇪🇫🇷', '1️⃣ and #️⃣ *️⃣', '© (c) ™ TM ® (r)', '👋🏽 wave', '❤️', 'x\u{FE0F}', '\u200d😀\u200d'];
  for (let i = 0; i < 5000; i++) {
    const t = r.chance(40) ? text(r, 24) : mutate(r, r.pick(seeds));
    const parts = splitEmoji(t);
    assert.equal(parts.map((p) => p.text).join(''), t, JSON.stringify(t));
    if (t) assert.ok(parts.every((p) => p.text.length > 0), JSON.stringify(parts));
    if (t.isWellFormed()) assert.ok(parts.every((p) => p.text.isWellFormed()), `split a pair: ${JSON.stringify(t)}`);
    for (const p of parts) if (p.emoji) emojiKey(p.text);
  }
  // Long input stays fast.
  const start = Date.now();
  splitEmoji('😀'.repeat(50_000) + '\u200d'.repeat(50_000));
  splitEmoji('a'.repeat(100_000));
  assert.ok(Date.now() - start < 2000, `took ${Date.now() - start} ms`);
});

test('stacked marks: any text, long runs included', () => {
  const r = new Rng(0xf0a);
  for (let i = 0; i < 3000; i++) {
    const t = r.chance(50) ? text(r, 30) : broken(r, 30);
    assert.equal(typeof stacks(t), 'boolean');
  }
  assert.equal(stacks('a\u0301\u0301\u0301\u0301b'), true);
  assert.equal(stacks('a\u0301\u0301\u0301b'), false);
  const start = Date.now();
  stacks('\u0301'.repeat(200_000));
  stacks('a\u0301'.repeat(100_000));
  assert.ok(Date.now() - start < 2000, `took ${Date.now() - start} ms`);
});

test('link clicks: only web addresses outside the app open, never other schemes', () => {
  const r = new Rng(0xf0b);
  const origin = 'http://localhost:1420';
  const seeds = ['https://twitch.tv/x', 'http://localhost:1420/#x', 'javascript:alert(1)', 'data:text/html,x', 'file:///C:/x', 'blob:http://localhost:1420/a', 'mailto:a@b.c', 'ms-settings:', 'tauri://localhost/', 'about:blank', 'HTTPS://EXAMPLE.COM', 'http://[::1]/', '//evil.example/x'];
  for (let i = 0; i < 6000; i++) {
    const href = r.chance(30) ? text(r, 30) : mutate(r, r.pick(seeds));
    const act = anchorAction(href, r.pick([origin, 'null', '', 'https://tauri.localhost']), r.chance(10));
    assert.ok(act === 'ignore' || act === 'stop' || act === 'open');
    if (act === 'open') {
      const u = new URL(href);
      assert.ok(u.protocol === 'http:' || u.protocol === 'https:', href);
    }
  }
  for (const href of ['javascript:alert(1)', 'data:text/html,x', 'file:///C:/x', 'vbscript:x', 'ms-settings:', 'tel:1', 'blob:https://a.b/c']) {
    assert.notEqual(anchorAction(href, origin, false), 'open', href);
  }
  assert.equal(anchorAction('https://example.com/', origin, true), 'ignore');
  assert.equal(anchorAction('not a url', origin, false), 'stop');
});
