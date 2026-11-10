import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import { READER_CHARS, READER_KEY, READER_LINES, readout, readReader, writeReader } from './readerMode.ts';

const line = (id: string, text: string, flags: string[] = []) =>
  ({ id, ts: 0, flags, sender: { id: 'u', login: 'u', display: 'U' }, body: [{ t: 'text', text }] }) as unknown as ChatMessage;

test('V110-A11 off by default, kept as 1, blocked storage means off', () => {
  const mem = new Map<string, string>();
  const store = { getItem: (k: string) => mem.get(k) ?? null, setItem: (k: string, v: string) => void mem.set(k, v), removeItem: (k: string) => void mem.delete(k) };
  assert.equal(readReader(store), false);
  writeReader(true, store);
  assert.equal(mem.get(READER_KEY), '1');
  assert.equal(readReader(store), true);
  writeReader(false, store);
  assert.equal(readReader(store), false);
  const bad = { getItem: () => { throw new Error('blocked'); }, setItem: () => { throw new Error('blocked'); }, removeItem: () => { throw new Error('blocked'); } };
  assert.equal(readReader(bad), false);
  assert.doesNotThrow(() => writeReader(true, bad));
});

test('V110-A11 the first readout is the newest line, later ones the lines since', () => {
  const list = [line('1', 'a'), line('2', 'b')];
  const first = readout(list, null);
  assert.equal(first.text, 'U: b');
  assert.equal(first.last, '2');
  assert.deepEqual(readout(list, '2'), { text: '', last: '2' });
  const more = [...list, line('3', 'c'), line('4', 'd')];
  assert.equal(readout(more, '2').text, 'U: c. U: d');
});

test('V110-A11 a burst reads only the newest few, and a trimmed anchor reads the newest line', () => {
  const many = Array.from({ length: 20 }, (_, i) => line(String(i), `m${i}`));
  const r = readout(many, '3');
  assert.equal(r.text.split('. ').length, READER_LINES);
  assert.ok(r.text.endsWith('m19'));
  assert.equal(readout(many, 'gone').text, 'U: m19');
});

test('V110-A11 deleted lines are skipped and a long readout is cut', () => {
  assert.equal(readout([line('1', 'a'), line('2', 'b', ['deleted'])], '1').text, '');
  const long = readout([line('1', 'x'.repeat(2000))], null).text;
  assert.equal(long.length, READER_CHARS);
});
