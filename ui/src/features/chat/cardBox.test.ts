import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { LinkPreview } from '../../ipc/bindings/LinkPreview';
import { CARD_HEIGHT, CARD_LINES, CARD_PIC, PREVIEW_PX, cardAfter, cardLines, cardPic, cardPicWidth, cardSite } from './cardBox.ts';

const info = (over: Partial<LinkPreview> = {}): LinkPreview =>
  ({
    kind: 'web',
    site: 'x',
    title: 'Title',
    subtitle: null,
    facts: [],
    lines: [],
    thumbnail: null,
    avatar: null,
    live: false,
    nsfw: false,
    finalUrl: null,
    ...over,
  }) as LinkPreview;

test('preview sizes: none, then 100, 200, 300 px (V110-C66)', () => {
  assert.deepEqual(PREVIEW_PX, { off: 0, small: 100, medium: 200, large: 300 });
  assert.ok(PREVIEW_PX.small < PREVIEW_PX.medium && PREVIEW_PX.medium < PREVIEW_PX.large);
});

test('the reserved box is one rule for the skeleton and the card', () => {
  assert.equal(CARD_HEIGHT, 'max(54px, 4.35em)');
  assert.deepEqual(CARD_PIC, { w: 96, h: 54 });
});

test('a card shows at most the lines it has room for', () => {
  const lines = cardLines(info({ subtitle: 'Sub', lines: [[{ text: 'More', bold: false }]] }));
  assert.equal(lines.length, CARD_LINES - 1);
  assert.deepEqual(lines, ['Title', 'Sub']);
  assert.deepEqual(cardLines(null), []);
});

test('V110-U70 a web card: subtitle or text second, facts on the site line', () => {
  const facts = [{ t: 'views', n: 1200 }, { t: 'likes', n: 41 }] as LinkPreview['facts'];
  const text = [[{ text: 'About things', bold: false }]];
  assert.deepEqual(cardLines(info({ subtitle: 'Chan', facts, lines: text })), ['Title', 'Chan']);
  assert.deepEqual(cardLines(info({ facts, lines: text })), ['Title', 'About things']);
  assert.equal(cardSite(info({ facts }), 'x.com', 'X'), 'x.com · 1.200 views · 41 likes');
  assert.equal(cardSite(info(), 'x.com', 'X'), 'x.com');
  assert.equal(cardSite(info({ kind: 'clip', facts }), 'twitch.tv', 'Twitch clip'), 'Twitch clip');
});

test('V110-U70 the picture is the preview, else the avatar, round', () => {
  const img = 'https://embed.cyex.app/v1/img?u=a';
  assert.deepEqual(cardPic(info({ thumbnail: img, avatar: img + 'b' })), { src: img, round: false });
  assert.deepEqual(cardPic(info({ avatar: img })), { src: img, round: true });
  assert.equal(cardPic(info({ avatar: 'https://pbs.twimg.com/a.jpg' })), null);
  assert.equal(cardAfter(info({ title: null, avatar: img }), false), 'ready');
});

test('V110-U70 a picture keeps its shape at the card height, never cropped', () => {
  assert.equal(cardPicWidth(1920, 1080), 96);
  assert.equal(cardPicWidth(3840, 700), 168);
  assert.equal(cardPicWidth(500, 1000), 40);
  assert.equal(cardPicWidth(0, 0), CARD_PIC.w);
});

test('an answer with something to show is ready', () => {
  assert.equal(cardAfter(info(), false), 'ready');
  assert.equal(cardAfter(info({ title: null, thumbnail: 'https://braize.pajlada.com/chatterino/thumbnail/x' }), true), 'ready');
  assert.equal(cardAfter(info({ title: null, thumbnail: 'https://a/b.png' }), true), 'plain');
});

test('no answer: the box stays on screen, goes off screen', () => {
  assert.equal(cardAfter(null, true), 'plain');
  assert.equal(cardAfter(null, false), 'gone');
  assert.equal(cardAfter(info({ title: null }), false), 'gone');
});
