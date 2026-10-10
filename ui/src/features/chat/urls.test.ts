import assert from 'node:assert/strict';
import { test } from 'node:test';

import { cheerImg, cheerTier, emoteImg, emoteLarge } from './urls.ts';

const cheer = {
  prefix: 'Cheer',
  template: 'https://x/actions/PREFIX/BACKGROUND/ANIMATION/TIER/SCALE.EXTENSION',
  tiers: [
    { bits: 1, color: '#979797' },
    { bits: 100, color: '#9c3ee8' },
    { bits: 1000, color: '#1db2a5' },
  ],
};

test('cheers use the highest tier they reach', () => {
  assert.equal(cheerTier(cheer, 99)?.bits, 1);
  assert.equal(cheerTier(cheer, 100)?.bits, 100);
  assert.equal(cheerTier(cheer, 5000)?.bits, 1000);
  assert.equal(cheerImg(cheer, cheerTier(cheer, 150)!, 'dark').src, 'https://x/actions/cheer/dark/animated/100/1.gif');
});

test('twitch emotes follow the theme', () => {
  const img = emoteImg('twitch', '25', 'light');
  assert.equal(img.src, 'https://static-cdn.jtvnw.net/emoticons/v2/25/default/light/1.0');
  assert.match(img.srcset, /3\.0 4x$/);
});

test('still frames for moving emotes', () => {
  assert.equal(emoteImg('twitch', '25', 'dark', { still: true }).src, 'https://static-cdn.jtvnw.net/emoticons/v2/25/static/dark/1.0');
  assert.equal(emoteImg('seventv', 'a', 'dark', { animated: true, still: true }).src, 'https://cdn.7tv.app/emote/a/1x_static.webp');
  // Not moving: 7TV has no still file for it.
  assert.equal(emoteImg('seventv', 'a', 'dark', { still: true }).src, 'https://cdn.7tv.app/emote/a/1x.webp');
  assert.equal(emoteLarge('ffz', '9', 'dark', { animated: true }), 'https://cdn.frankerfacez.com/emote/9/animated/4');
});
