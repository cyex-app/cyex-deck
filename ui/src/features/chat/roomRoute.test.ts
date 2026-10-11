import assert from 'node:assert/strict';
import { test } from 'node:test';

import { parseRoomRoute, roomRoute } from './roomRoute.ts';

test('#channel windows keep the account of the chat they came from (V102-05)', () => {
  assert.equal(roomRoute('Forsen', '123'), 'room/forsen/123');
  assert.equal(roomRoute('forsen', null), 'room/forsen');
  assert.deepEqual(parseRoomRoute('room/forsen/123'), { login: 'forsen', account: '123' });
  assert.deepEqual(parseRoomRoute('room/forsen'), { login: 'forsen', account: null });
  for (const bad of ['room/', 'room/Bad Name', 'room/forsen/abc', 'room/forsen/1/2', 'chat/forsen']) {
    assert.equal(parseRoomRoute(bad), null, bad);
  }
});
