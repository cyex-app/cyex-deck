import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import { READER_CHARS, READER_LINES, readout } from './readerMode.ts';

const line = (id: string, text: string, flags: string[] = []) =>
  ({ id, ts: 0, flags, sender: { id: 'u', login: 'u', display: 'U' }, body: [{ t: 'text', text }] }) as unknown as ChatMessage;

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

test('V110-A11 screen reader mode is a stored chat setting, not browser storage', async () => {
  const { readFileSync } = await import('node:fs');
  const { ENTRIES } = await import('../settings/registry/index.ts');
  assert.ok(ENTRIES.some((e) => e.id === 'chat.screenReader'));
  assert.ok(!ENTRIES.some((e) => e.id === 'chat:reader'));
  assert.match(readFileSync(new URL('../../ipc/bindings/Chat.ts', import.meta.url), 'utf8'), /screenReader: boolean/);
  for (const file of ['MessageList.svelte', 'readerMode.ts']) {
    assert.doesNotMatch(readFileSync(new URL(file, import.meta.url), 'utf8'), /localStorage|cyex\.chatReader/);
  }
});
