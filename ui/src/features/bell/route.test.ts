import assert from 'node:assert/strict';
import { test } from 'node:test';

import { route } from './route.ts';

test('channels open as tabs', () => {
  assert.deepEqual(route('https://www.twitch.tv/xqc'), { kind: 'channel', login: 'xqc' });
  assert.deepEqual(route('https://twitch.tv/Some_One/videos'), { kind: 'channel', login: 'some_one' });
  assert.deepEqual(route('/xqc'), { kind: 'channel', login: 'xqc' });
});

test('other twitch pages stay in the app', () => {
  assert.equal(route('https://www.twitch.tv/drops/inventory').kind, 'twitch');
  assert.equal(route('https://www.twitch.tv/videos/123').kind, 'twitch');
  assert.equal(route('https://www.twitch.tv/xqc/clip/Abc-1').kind, 'twitch');
  assert.equal(route('https://dashboard.twitch.tv/u/x').kind, 'twitch');
});

test('the rest goes to the browser', () => {
  assert.equal(route('https://example.com/a').kind, 'web');
  assert.equal(route('https://twitch.tv.evil.com/x').kind, 'web');
  assert.equal(route('http://www.twitch.tv/xqc').kind, 'web');
});
