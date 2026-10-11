import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Pinned } from '../../ipc/bindings/Pinned';
import type { VotesUpdate } from '../../ipc/bindings/VotesUpdate';
import {
  chatEvents,
  close,
  EMPTY_STRIP,
  eventsLabel,
  hide,
  reopen,
  settle,
  shows,
  type ChatEvent,
} from './eventStrip.ts';

const pin: ChatEvent = { key: 'pin:1', live: true };
const poll: ChatEvent = { key: 'poll:2', live: true };
const pred: ChatEvent = { key: 'prediction:3', live: true };
const ended = (e: ChatEvent): ChatEvent => ({ ...e, live: false });

test('the label says Event for one, Events for more (V103-04)', () => {
  assert.equal(eventsLabel(1), '1 Event');
  assert.equal(eventsLabel(2), '2 Events');
});

test('hidden items count; the button opens them all again', () => {
  let s = settle(EMPTY_STRIP, [pin, poll]);
  s = hide(s, pin);
  s = hide(s, poll);
  assert.equal(s.hidden.size, 2);
  assert.ok(!shows(s, pin.key));
  s = reopen(s);
  assert.equal(s.hidden.size, 0);
  assert.ok(shows(s, pin.key) && shows(s, poll.key));
});

test('an item that ended leaves the count and stays closed', () => {
  let s = settle(EMPTY_STRIP, [pin, poll]);
  s = hide(hide(s, pin), poll);
  s = settle(s, [pin, ended(poll)]);
  assert.deepEqual([...s.hidden], [pin.key]);
  assert.ok(!shows(reopen(s), poll.key), 'its result must not pop up');
  // A pin taken down is simply gone.
  s = settle(s, [ended(poll)]);
  assert.equal(s.hidden.size, 0);
});

test('new items join the button while it shows, and show normally when it does not', () => {
  let s = hide(settle(EMPTY_STRIP, [pin]), pin);
  s = settle(s, [pin, poll]);
  assert.equal(s.hidden.size, 2);
  s = reopen(s);
  s = settle(s, [pin, poll, pred]);
  assert.equal(s.hidden.size, 0);
  assert.ok(shows(s, pred.key));
});

test('only what is still going can be hidden; close removes it from the count', () => {
  let s = settle(EMPTY_STRIP, [ended(poll), pin]);
  assert.equal(hide(s, ended(poll)), s);
  s = close(hide(s, pin), pin.key);
  assert.equal(s.hidden.size, 0);
  assert.ok(!shows(s, pin.key));
});

test('nothing changed keeps the same state', () => {
  const s = hide(settle(EMPTY_STRIP, [pin, poll]), pin);
  assert.equal(settle(s, [pin, poll]), s);
});

test('a timed pin ends with its time; predictions end once settled', () => {
  const at = Date.parse('2026-10-11T12:00:00Z');
  const p = { id: '1', endsAt: '2026-10-11T12:00:00Z' } as Pinned;
  const v = {
    poll: { id: '2', status: 'COMPLETED' },
    prediction: { id: '3', status: 'LOCKED' },
  } as VotesUpdate;
  assert.deepEqual(chatEvents(p, v, at - 1), [
    { key: 'pin:1', live: true },
    { key: 'poll:2', live: false },
    { key: 'prediction:3', live: true },
  ]);
  const resolved = { ...v, prediction: { ...v.prediction!, status: 'RESOLVED' } } as VotesUpdate;
  assert.deepEqual(
    chatEvents(p, resolved, at).map((e) => e.live),
    [false, false, false],
  );
  assert.ok(chatEvents({ id: '1', endsAt: null } as Pinned, undefined, at)[0].live);
});
