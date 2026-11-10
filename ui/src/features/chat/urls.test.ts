import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { pictureScale, type Scale } from '../../lib/scale.ts';
import { badgeLarge, badgeSet, badgeSrc,cheerImg, cheerTier, emoteImg, emoteLarge } from './urls.ts';

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
  assert.equal(cheerImg(cheer, cheerTier(cheer, 150)!, 'dark', 1).src, 'https://x/actions/cheer/dark/animated/100/1.gif');
});

test('twitch emotes follow the theme', () => {
  const img = emoteImg('twitch', '25', 'light', { scale: 1 });
  assert.equal(img.src, 'https://static-cdn.jtvnw.net/emoticons/v2/25/default/light/1.0');
  assert.ok(emoteImg('twitch', '25', 'light', { scale: 4 }).src.endsWith('/3.0'));
});

test('the <3 hearts stop at 2.0, where their drawing still matches', () => {
  assert.ok(emoteImg('twitch', '9', 'dark', { scale: 4 }).src.endsWith('/9/default/dark/2.0'));
  assert.ok(emoteImg('twitch', '555555584', 'light', { scale: 2 }).src.endsWith('/2.0'));
  assert.ok(emoteImg('twitch', '555555584', 'dark', { scale: 1 }).src.endsWith('/1.0'));
});

test('one url per picture, from the screen density (V105-08)', () => {
  assert.ok(emoteImg('bttv', 'b', 'dark', { scale: 4 }).src.endsWith('/3x.webp'));
  assert.equal(emoteImg('seventv', 'a', 'dark', { scale: 2 }).src, 'https://cdn.7tv.app/emote/a/2x.webp');
  assert.equal(emoteImg('seventv', 'a', 'dark', { scale: 1 }).src, 'https://cdn.7tv.app/emote/a/1x.webp');
  assert.ok(cheerImg(cheer, cheerTier(cheer, 150)!, 'dark', 2).src.endsWith('/2.gif'));
  const b = { image1x: 'a', image2x: 'b', image4x: 'c' };
  assert.deepEqual([1, 2, 4].map((s) => badgeSrc(b, s as Scale)), ['a', 'b', 'c']);
});

test('badges give a 1x/2x/4x srcset and the preview takes the largest (V110-B1)', () => {
  const b = { image1x: 'a', image2x: 'b', image4x: 'c' };
  assert.equal(badgeSet(b), 'a 1x, b 2x, c 4x');
  assert.equal(badgeLarge(b), 'c');
  // A badge with one picture for every size has nothing to choose from.
  assert.equal(badgeSet({ image1x: 'a', image2x: 'a', image4x: 'a' }), undefined);
  assert.equal(badgeSet({ image1x: 'a', image2x: '', image4x: 'c' }), 'a 1x, c 4x');
  assert.equal(badgeLarge({ image1x: 'a', image2x: 'b', image4x: '' }), 'b');
  // A cyex badge with a 256 px picture previews with it; null or empty falls back to the 4x.
  assert.equal(badgeLarge({ ...b, imageLarge: 'L' }), 'L');
  assert.equal(badgeLarge({ ...b, imageLarge: null }), 'c');
  assert.equal(badgeLarge({ ...b, imageLarge: '' }), 'c');
  const src = readFileSync(new URL('./Badges.svelte', import.meta.url), 'utf8');
  assert.ok(!src.includes('data-tip-img={b.image4x}'));
  assert.ok((src.match(/srcset=/g) ?? []).length >= 2);
});

test('still frames for moving emotes', () => {
  assert.equal(emoteImg('twitch', '25', 'dark', { still: true, scale: 1 }).src, 'https://static-cdn.jtvnw.net/emoticons/v2/25/static/dark/1.0');
  assert.equal(emoteImg('seventv', 'a', 'dark', { animated: true, still: true, scale: 1 }).src, 'https://cdn.7tv.app/emote/a/1x_static.webp');
  // Not moving: 7TV has no still file for it.
  assert.equal(emoteImg('seventv', 'a', 'dark', { still: true, scale: 1 }).src, 'https://cdn.7tv.app/emote/a/1x.webp');
  assert.equal(emoteLarge('ffz', '9', 'dark', { animated: true }), 'https://cdn.frankerfacez.com/emote/9/animated/4');
});

test('cyex badges pick their picture by screen density like every badge (V109-F82)', () => {
  const at = (k: string, e: string) => ({
    image1x: `https://api.cyex.app/badges/v1/img/${k}/1x.${e}`,
    image2x: `https://api.cyex.app/badges/v1/img/${k}/2x.${e}`,
    image4x: `https://api.cyex.app/badges/v1/img/${k}/4x.${e}`,
  });
  for (const e of ['png', 'webp']) {
    const b = at('supporter', e);
    assert.equal(badgeSrc(b, pictureScale(1)), b.image1x);
    assert.equal(badgeSrc(b, pictureScale(1.5)), b.image2x);
    assert.equal(badgeSrc(b, pictureScale(2)), b.image2x);
    assert.equal(badgeSrc(b, pictureScale(3)), b.image4x);
  }
  const custom = at('u/1294600497/h', 'png');
  assert.ok(badgeSrc(custom, pictureScale(1.5)).endsWith('/2x.png'));
});
