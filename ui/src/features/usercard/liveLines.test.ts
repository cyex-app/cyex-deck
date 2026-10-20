import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import { changedLines, theirLines } from './liveLines.ts';

const line = (id: string, user: string, notice = false) =>
  ({ id, sender: { id: user }, notice: notice ? {} : null }) as unknown as ChatMessage;

test('theirLines keeps only that user\'s chat lines', () => {
  const a = line('1', 'u');
  const list = [a, line('2', 'v'), line('3', 'u', true)];
  assert.deepEqual(theirLines(list, 'u'), [a]);
});

test('changedLines sees a new or replaced line, not the same ones', () => {
  const a = line('1', 'u');
  const b = line('2', 'u');
  assert.equal(changedLines([a], [a]), false);
  assert.equal(changedLines([a], [a, b]), true);
  assert.equal(changedLines([a, b], [a, line('2', 'u')]), true);
  assert.equal(changedLines([], []), false);
});
