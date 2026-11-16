import assert from 'node:assert/strict';
import { test } from 'node:test';

import { height1x, sizeShare, smallShare } from './drawSize.ts';

test('V111-44 wider than 3:1 draws shorter, as 7TV serves it', () => {
  assert.equal(sizeShare('seventv', [32, 32]), 1);
  assert.equal(sizeShare('seventv', [96, 32]), 1);
  // digga: 7TV's 1x box is 96 × 18.
  assert.equal(sizeShare('seventv', [96, 18]), 18 / 32);
  // An FFZ 128 × 32 is drawn 3:1 wide at most, by height.
  assert.equal(sizeShare('ffz', [128, 32]), 0.75);
  assert.equal(sizeShare('seventv', null), 1);
  assert.equal(sizeShare('twitch', [28, 28]), 1);
});

test('V111-44 small pictures draw at their own size', () => {
  assert.equal(smallShare('seventv', 7), 7 / 32);
  assert.equal(smallShare('seventv', 30), null);
  assert.equal(smallShare('bttv', 14), 0.5);
  assert.equal(smallShare('twitch', 10), null);
  assert.equal(sizeShare('bttv', [14, 14]), 0.5);
  // Small and wide: the smaller share wins.
  assert.equal(sizeShare('ffz', [80, 10]), 10 / 32);
});

test('V111-44 1x height from the CDN scale', () => {
  assert.equal(height1x('seventv', 'https://cdn.7tv.app/emote/01ABC/2x.webp', 14), 7);
  assert.equal(height1x('seventv', 'https://cdn.7tv.app/emote/01ABC/4x_static.webp', 128), 32);
  assert.equal(height1x('bttv', 'https://cdn.betterttv.net/emote/abc/static/3x.webp', 84), 28);
  assert.equal(height1x('ffz', 'https://cdn.frankerfacez.com/emote/123/animated/2', 36), 18);
  assert.equal(height1x('seventv', 'blob:saved-copy', 64), null);
  assert.equal(height1x('seventv', 'https://cdn.7tv.app/emote/01ABC/1x.webp', 0), null);
});
