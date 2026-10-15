import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Kind, Picked, SelectError } from '../../ipc/cosmetics';
import { Selection, type Scheduler } from './select.ts';

class Fake implements Scheduler {
  now = 0;
  private list: { id: number; at: number; fn: () => void }[] = [];
  private n = 0;
  set(fn: () => void, ms: number) {
    this.list.push({ id: ++this.n, at: this.now + ms, fn });
    return this.n;
  }
  clear(id: unknown) {
    this.list = this.list.filter((t) => t.id !== id);
  }
  tick(ms: number) {
    this.now += ms;
    for (;;) {
      const due = this.list.filter((t) => t.at <= this.now).sort((a, b) => a.at - b.at)[0];
      if (!due) return;
      this.list = this.list.filter((t) => t !== due);
      due.fn();
    }
  }
}
const flush = () => new Promise((r) => setImmediate(r));
const done = (id: string | null): Picked => ({ type: 'done', id });

function make(reply: (k: Kind, id: string | null) => Promise<Picked> = async (_k, id) => done(id)) {
  const sched = new Fake();
  const sent: [Kind, string | null][] = [];
  const s = new Selection(
    (k, id) => {
      sent.push([k, id]);
      return reply(k, id);
    },
    () => {},
    400,
    sched,
  );
  s.reset('p0', 'b0');
  return { s, sched, sent };
}

test('quick picks send once with the last id', async () => {
  const { s, sched, sent } = make();
  s.pick('paint', 'a');
  sched.tick(100);
  s.pick('paint', 'b');
  sched.tick(100);
  s.pick('paint', 'c');
  assert.equal(s.shown.paint, 'c');
  sched.tick(399);
  assert.equal(sent.length, 0);
  sched.tick(1);
  await flush();
  assert.deepEqual(sent, [['paint', 'c']]);
  assert.equal(s.status, 'saved');
  assert.equal(s.confirmed.paint, 'c');
});

test('failure rolls back', async () => {
  const err: SelectError = { type: 'notAllowed' };
  const { s, sched } = make(() => Promise.reject(err));
  s.pick('badge', 'x');
  sched.tick(400);
  await flush();
  assert.equal(s.shown.badge, 'b0');
  assert.deepEqual(s.status, { error: err });
});

test('superseded keeps the newer pick', async () => {
  let first: (p: Picked) => void = () => {};
  let n = 0;
  const { s, sched, sent } = make(() =>
    n++ === 0 ? new Promise<Picked>((r) => (first = r)) : Promise.resolve(done('b')),
  );
  s.pick('paint', 'a');
  sched.tick(400);
  s.pick('paint', 'b');
  first({ type: 'superseded' });
  await flush();
  assert.equal(s.shown.paint, 'b');
  assert.notEqual(s.status, 'saved');
  sched.tick(400);
  await flush();
  assert.equal(sent.length, 2);
  assert.equal(s.confirmed.paint, 'b');
});

test('undo sends the old id, and expires', async () => {
  const { s, sched, sent } = make();
  s.pick('paint', 'a');
  sched.tick(400);
  await flush();
  assert.deepEqual(s.undoable, { kind: 'paint', id: 'p0' });
  s.undo();
  sched.tick(400);
  await flush();
  assert.deepEqual(sent[1], ['paint', 'p0']);
  assert.equal(s.confirmed.paint, 'p0');
  sched.tick(5000);
  assert.equal(s.undoable, null);
  assert.equal(s.status, 'idle');
});

test('paint and badge timers are independent', async () => {
  const { s, sched, sent } = make();
  s.pick('paint', 'a');
  sched.tick(200);
  s.pick('badge', 'z');
  sched.tick(200);
  await flush();
  assert.deepEqual(sent, [['paint', 'a']]);
  sched.tick(200);
  await flush();
  assert.deepEqual(sent, [
    ['paint', 'a'],
    ['badge', 'z'],
  ]);
  assert.equal(s.status, 'saved');
});
