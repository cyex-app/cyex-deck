import assert from 'node:assert/strict';
import { test } from 'node:test';

import { lineKey, lineLabel, revealDelta, stepLine } from './lineSelect.ts';

const ids = ['a', 'b', 'c'];

test('V108-F54 Alt+Up with nothing selected takes the newest line', () => {
  assert.equal(stepLine(ids, null, -1), 'c');
});

test('V108-F54 Alt+Up walks to older lines and stops at the oldest', () => {
  assert.equal(stepLine(ids, 'c', -1), 'b');
  assert.equal(stepLine(ids, 'b', -1), 'a');
  assert.equal(stepLine(ids, 'a', -1), 'a');
});

test('V108-F54 Alt+Down walks to newer lines and ends past the newest', () => {
  assert.equal(stepLine(ids, 'a', 1), 'b');
  assert.equal(stepLine(ids, 'c', 1), null);
});

test('V108-F54 Alt+Down with nothing selected stays out', () => {
  assert.equal(stepLine(ids, null, 1), null);
});

test('V108-F54 a selected line that left the list starts again from the newest', () => {
  assert.equal(stepLine(ids, 'gone', -1), 'c');
  assert.equal(stepLine(ids, 'gone', 1), null);
});

test('V108-F54 an empty chat selects nothing', () => {
  assert.equal(stepLine([], null, -1), null);
  assert.equal(stepLine([], 'a', 1), null);
});

test('V108-F54 a line in view needs no scroll', () => {
  assert.equal(revealDelta({ top: 50, bottom: 70 }, { top: 0, bottom: 200 }, 12), 0);
});

test('V108-F54 a line near the top edge scrolls up to leave room for its buttons', () => {
  assert.equal(revealDelta({ top: 4, bottom: 24 }, { top: 0, bottom: 200 }, 12), -8);
  assert.equal(revealDelta({ top: -30, bottom: -10 }, { top: 0, bottom: 200 }, 12), -42);
});

test('V108-F54 a line below the box scrolls down just enough', () => {
  assert.equal(revealDelta({ top: 190, bottom: 230 }, { top: 0, bottom: 200 }, 12), 30);
});

test('V108-F54 a line taller than the box shows its top', () => {
  assert.equal(revealDelta({ top: 100, bottom: 600 }, { top: 0, bottom: 200 }, 12), 88);
});

const key = (k: string, mods: Partial<Record<'altKey' | 'ctrlKey' | 'metaKey' | 'shiftKey', boolean>> = {}) => ({
  key: k,
  altKey: false,
  ctrlKey: false,
  metaKey: false,
  shiftKey: false,
  ...mods,
});

test('V108-F54 keys on a selected line', () => {
  assert.equal(lineKey(key('Enter')), 'reply');
  assert.equal(lineKey(key('r')), 'reply');
  assert.equal(lineKey(key('P')), 'pin');
  assert.equal(lineKey(key('m')), 'menu');
  assert.equal(lineKey(key('ContextMenu')), 'menu');
  assert.equal(lineKey(key('F10', { shiftKey: true })), 'menu');
  assert.equal(lineKey(key('Escape')), 'clear');
});

test('V108-F54 shortcuts with Ctrl, Alt or Meta and other keys are not ours', () => {
  assert.equal(lineKey(key('r', { ctrlKey: true })), null);
  assert.equal(lineKey(key('m', { altKey: true })), null);
  assert.equal(lineKey(key('p', { metaKey: true })), null);
  assert.equal(lineKey(key('x')), null);
  assert.equal(lineKey(key('F10')), null);
});

test('V108-F54 a screen reader hears name and text, or the notice', () => {
  const body = [{ t: 'text', text: 'hello ' }, { t: 'emote', name: 'Kappa' }] as never;
  assert.equal(lineLabel({ sender: { display: 'Kuni' } as never, body, notice: null }), 'Kuni: hello Kappa');
  assert.equal(lineLabel({ sender: null, body: [], notice: { text: 'Chat was cleared' } as never }), 'Chat was cleared');
});
