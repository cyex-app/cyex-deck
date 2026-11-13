import assert from 'node:assert/strict';
import { test } from 'node:test';
import { isSmiley } from './smiley.ts';

const url = (s: number) => `https://static-cdn.jtvnw.net/emoticons/v2/1/default/dark/${s}.0`;

test('V110-N18 small Twitch pictures are smileys, full ones are emotes', () => {
  assert.equal(isSmiley(url(1), 18), true);
  assert.equal(isSmiley(url(2), 36), true);
  assert.equal(isSmiley(url(3), 72), true);
  assert.equal(isSmiley(url(1), 28), false);
  assert.equal(isSmiley(url(2), 56), false);
  assert.equal(isSmiley(url(3), 112), false);
  assert.equal(isSmiley('https://cdn.7tv.app/emote/x/1x.webp', 10), false, 'only Twitch');
  assert.equal(isSmiley(url(1), 0), false, 'not loaded');
});
