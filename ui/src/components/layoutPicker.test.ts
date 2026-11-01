import assert from 'node:assert/strict';
import { test } from 'node:test';
import { barSplit, BODY, chatSplit, sayPick, sideForKey, SIDES, tabZone, chatZone } from './layoutPicker.ts';

const area = (r: { w: number; h: number }) => r.w * r.h;

test('bar and rest tile the body for every side', () => {
  for (const s of SIDES) {
    const { bar, rest } = barSplit(s);
    assert.equal(area(bar) + area(rest), area(BODY), s);
  }
});

test('chat and player tile the area; chat hugs its side', () => {
  for (const s of SIDES) {
    const { rest } = barSplit('left');
    const { chat, player } = chatSplit(s, rest);
    assert.equal(area(chat) + area(player), area(rest), s);
  }
  const { rest } = barSplit('top');
  assert.equal(chatSplit('left', rest).chat.x, rest.x);
  const r = chatSplit('right', rest).chat;
  assert.equal(r.x + r.w, rest.x + rest.w);
  assert.equal(chatSplit('top', rest).chat.y, rest.y);
});

test('arrow keys name the side they point at', () => {
  assert.equal(sideForKey('ArrowLeft'), 'left');
  assert.equal(sideForKey('ArrowUp'), 'top');
  assert.equal(sideForKey('ArrowDown'), 'bottom');
  assert.equal(sideForKey('ArrowRight'), 'right');
  assert.equal(sideForKey('Enter'), null);
});

test('every side has a zone for tabs and chat', () => {
  for (const s of SIDES) {
    assert.equal(tabZone(s).length, 4);
    assert.equal(chatZone(s).length, 3);
  }
});

test('captions', () => {
  assert.equal(sayPick('tabs', 'left'), 'Tabs on the left');
  assert.equal(sayPick('chat', 'bottom'), 'Chat at the bottom');
});
