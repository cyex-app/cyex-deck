import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { CheerSent } from '../../ipc/bindings/CheerSent';
import { ChatSend, leftOver, type CheerAsk, type Line } from './cheerSend.ts';

/** Fake ports: every call is logged; Rust's answers come from `replies`. */
function fake(replies: (CheerSent | Error)[] = [], sendFails = false) {
  const log: string[] = [];
  const cheers: [string, number, boolean, boolean][] = [];
  let open: CheerAsk | null = null;
  let release: (() => void) | null = null;
  let hold = false;
  const chat = new ChatSend({
    cheer: async (value, bits, confirmed, careful) => {
      cheers.push([value, bits, confirmed, careful]);
      if (hold) await new Promise<void>((r) => (release = r));
      const r = replies.shift() ?? { kind: 'text' };
      if (r instanceof Error) throw r;
      return r;
    },
    send: async (value) => {
      log.push(`send ${value}`);
      if (sendFails) throw new Error('nope');
    },
    ask: (a) => {
      open = a;
      log.push(`ask ${a.bits} again=${a.again}`);
    },
    asking: () => open !== null,
    wentOut: (_raw, value, keep) => log.push(`out ${value} keep=${keep}`),
    balance: (b) => log.push(`balance ${b}`),
    bitsOff: () => log.push('bitsOff'),
    ok: () => log.push('ok'),
    fail: (e) => log.push(`fail ${(e as Error).message}`),
  });
  return {
    chat,
    log,
    cheers,
    get open() {
      return open;
    },
    close: () => (open = null),
    hold: () => (hold = true),
    release: () => release?.(),
  };
}

const line = (value: string, bits: number, keep = false): Line => ({ raw: `${value} `, value, bits, keep });

test('plain chat goes out and leaves the box (P-01)', async () => {
  const f = fake();
  await f.chat.submit(line('hi', 0));
  assert.deepEqual(f.log, ['send hi', 'ok', 'out hi keep=false']);
  assert.equal(f.cheers.length, 0);
});

test('a failed plain send keeps the text and frees the box (P-01)', async () => {
  const f = fake([], true);
  await f.chat.submit(line('hi', 0));
  assert.deepEqual(f.log, ['send hi', 'fail nope']);
  assert.equal(f.chat.busy, false);
});

test('a cheer that Rust sends: balance, then out with the reply cleared (P-01)', async () => {
  const f = fake([{ kind: 'done', bits: 100, balance: 400 }]);
  await f.chat.submit(line('Cheer100 gg', 100, true));
  assert.deepEqual(f.cheers, [['Cheer100 gg', 100, false, false]]);
  assert.deepEqual(f.log, ['balance 400', 'ok', 'out Cheer100 gg keep=true']);
});

test('over the threshold Rust asks; yes sends it confirmed (P-01)', async () => {
  const f = fake([
    { kind: 'ask', bits: 500, again: false },
    { kind: 'done', bits: 500, balance: null },
  ]);
  await f.chat.submit(line('Cheer500', 500));
  assert.deepEqual(f.log, ['ok', 'ask 500 again=false']);
  const a = f.open!;
  assert.equal(a.value, 'Cheer500');
  f.close();
  await f.chat.confirm(a);
  assert.deepEqual(f.cheers[1], ['Cheer500', 500, true, false]);
  assert.deepEqual(f.log.slice(2), ['balance null', 'ok', 'out Cheer500 keep=false']);
});

test('the same cheer again asks with again=true (P-01)', async () => {
  const f = fake([{ kind: 'ask', bits: 1, again: true }]);
  await f.chat.submit(line('Cheer1', 1));
  assert.equal(f.open?.again, true);
});

test('pasted or recalled cheers go to Rust as careful (P-01)', async () => {
  const f = fake([{ kind: 'ask', bits: 1, again: false }]);
  await f.chat.submit(line('Cheer1', 1), false, true);
  assert.deepEqual(f.cheers, [['Cheer1', 1, false, true]]);
});

test('a held Enter never cheers, nor while "Spend N Bits?" is open (P-01)', async () => {
  const f = fake([{ kind: 'ask', bits: 1, again: false }]);
  await f.chat.submit(line('Cheer1', 1), true);
  assert.equal(f.cheers.length, 0);
  await f.chat.submit(line('Cheer1', 1));
  await f.chat.submit(line('Cheer1', 1));
  assert.equal(f.cheers.length, 1);
  // A held Enter still sends plain chat.
  await f.chat.submit(line('hi', 0), true);
  assert.deepEqual(f.log.at(-3), 'send hi');
});

test('Bits off: the gem goes and the line goes out as plain chat (P-01)', async () => {
  const f = fake([{ kind: 'text' }]);
  await f.chat.submit(line('Cheer1 hi', 1));
  assert.deepEqual(f.log, ['bitsOff', 'send Cheer1 hi', 'ok', 'out Cheer1 hi keep=false']);
});

test('a failed cheer says why and keeps the text (P-01)', async () => {
  const f = fake([new Error('Couldn’t reach Twitch.')]);
  await f.chat.submit(line('Cheer1', 1));
  assert.deepEqual(f.log, ['fail Couldn’t reach Twitch.']);
  assert.equal(f.chat.busy, false);
});

test('a second Enter while a cheer is on its way does nothing (P-01)', async () => {
  const f = fake([{ kind: 'done', bits: 1, balance: 9 }]);
  f.hold();
  const first = f.chat.submit(line('Cheer1', 1));
  assert.equal(f.chat.busy, true);
  await f.chat.submit(line('Cheer1', 1));
  await f.chat.submit(line('hi', 0));
  await f.chat.confirm({ ...line('Cheer1', 1), again: false });
  await Promise.resolve();
  f.release();
  await first;
  assert.equal(f.cheers.length, 1);
  assert.deepEqual(f.log, ['balance 9', 'ok', 'out Cheer1 keep=false']);
});

test('what was typed while sending stays in the box (P-01)', () => {
  assert.equal(leftOver('hi ', 'hi ', 'hi'), '');
  assert.equal(leftOver('hi there', 'hi ', 'hi'), 'there');
  assert.equal(leftOver('changed', 'hi ', 'hi'), 'changed');
});
