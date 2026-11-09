import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { WhisperState } from '../../ipc/bindings/WhisperState';
import type { WhisperThread } from '../../ipc/bindings/WhisperThread';
import { applyChange } from './threadChange.ts';

const thread = (id: string, lastTs: number): WhisperThread =>
  ({ id, lastTs, unread: 0, archived: false, muted: false, spam: false, temporary: false, lastText: '', lastOwn: false }) as WhisperThread;
const state = (threads: WhisperThread[]): WhisperState =>
  ({ account: null, history: true, threads, more: true, loading: false, blockStrangers: false, unread: 0 }) as WhisperState;

test('V110-F73 a changed thread replaces its old self and moves to the top', () => {
  const next = applyChange(state([thread('a', 30), thread('b', 20)]), { thread: thread('b', 40), dropped: [], unread: 2 });
  assert.deepEqual(next.threads.map((t) => t.id), ['b', 'a']);
  assert.equal(next.unread, 2);
  assert.equal(next.more, true, 'the rest of the state stays');
});

test('V110-F73 a new thread is added and the cut ones leave', () => {
  const next = applyChange(state([thread('a', 30), thread('b', 20)]), { thread: thread('c', 50), dropped: ['b'], unread: 1 });
  assert.deepEqual(next.threads.map((t) => t.id), ['c', 'a']);
});
