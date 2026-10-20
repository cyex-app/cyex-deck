import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import { goneNote, markGone, mergeGone } from './gone.ts';

const line = (extra: Partial<ChatMessage> = {}): ChatMessage => ({ id: 'a', ts: 0, body: [], flags: [], ...extra }) as ChatMessage;

test('V107-R34 the note says how, and which mod when known', () => {
  assert.equal(goneNote({ how: 'deleted' }), '(deleted)');
  assert.equal(goneNote({ how: 'timedOut' }), '(timed out)', 'length unknown');
  assert.equal(goneNote({ how: 'timedOut', secs: 600 }), '(timed out 600s)');
  assert.equal(goneNote({ how: 'timedOut', secs: 600, by: 'x' }), '(timed out 600s by x)');
  assert.equal(goneNote({ how: 'timedOut', secs: 86400 }), '(timed out 86.400s)');
  assert.equal(goneNote({ how: 'banned', by: 'Kuni' }), '(banned by Kuni)');
  assert.equal(goneNote({ how: 'deleted', by: 'Mod B' }), '(deleted by Mod B)');
  assert.equal(goneNote(undefined), '', 'a plugin hid it: no note');
});

test("V107-R34 IRC's copy keeps the mod a feed named; a ban replaces a timeout", () => {
  const feed = { how: 'timedOut' as const, by: 'Kuni' };
  assert.deepEqual(mergeGone({ how: 'timedOut' }, feed), feed);
  assert.deepEqual(mergeGone({ how: 'timedOut', secs: 600 }, { ...feed, secs: 598 }), { ...feed, secs: 600 });
  assert.deepEqual(mergeGone({ how: 'timedOut', secs: 60 }, { ...feed, secs: 600 }), { how: 'timedOut', secs: 60 }, 'a new timeout');
  assert.deepEqual(mergeGone({ how: 'banned' }, feed), { how: 'banned' });
});

test('V107-R34 marking: once, then again only with news', () => {
  const m = markGone(line(), { how: 'timedOut' });
  assert.ok(m && m.flags.includes('deleted') && m.gone?.how === 'timedOut');
  assert.equal(markGone(m, { how: 'timedOut' }), null);
  const by = markGone(m, { how: 'timedOut', by: 'Kuni' });
  assert.equal(by?.gone?.by, 'Kuni');
  assert.deepEqual(by?.flags, ['deleted']);
  const hidden = markGone(line(), undefined);
  assert.ok(hidden && hidden.flags.includes('deleted') && !hidden.gone);
  assert.equal(markGone(hidden, undefined), null);
});
