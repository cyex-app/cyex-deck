import assert from 'node:assert/strict';
import { test } from 'node:test';

import { hitInside } from './shell.ts';

interface Fake {
  parentElement: Fake | null;
  matches(sel: string): boolean;
}

function el(classes: string, parent: Fake | null = null): Fake {
  const list = classes.split(' ');
  return { parentElement: parent, matches: (sel: string) => sel.split(', ').some((s) => list.includes(s.slice(1))) };
}

test('only the player counts, not the app around it', () => {
  // The app's own `.body.top` holds the player (V101-27).
  const app = el('body top');
  const shell = el('shell', app);
  const video = el('video', shell);
  const bar = el('bar', shell);
  const button = el('ctl', bar);
  assert.equal(hitInside(video, shell, '.bar, .top'), false);
  assert.equal(hitInside(button, shell, '.bar, .top'), true);
  assert.equal(hitInside(shell, shell, '.shell'), false, 'the player itself is not a control');
  assert.equal(hitInside(null, shell, '.bar'), false);
});
