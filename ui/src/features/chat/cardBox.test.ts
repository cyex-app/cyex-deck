import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { LinkPreview } from '../../ipc/bindings/LinkPreview';
import { CARD_HEIGHT, CARD_LINES, CARD_PIC, PREVIEW_PX, cardAfter, cardLines } from './cardBox.ts';

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
