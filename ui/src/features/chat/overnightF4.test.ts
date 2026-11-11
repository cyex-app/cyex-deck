import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

// Source scans for the N2 fixes (overnight 2026/11/11).
const read = (p: string) => readFileSync(new URL(p, import.meta.url), 'utf8');

test('N2-1 the theatre chat button stays while hovered and its moves do not re-arm the rest timer', () => {
  const s = read('../channel/StreamSide.svelte');
  assert.match(s, /onpointermove=\{\(e\) => e\.stopPropagation\(\)\}/);
  assert.match(s, /\.chat-toggle:hover,\s*\.chat-toggle:focus-visible \{\s*opacity: 1;/);
  assert.match(s, /onpointerleave=\{stir\}/);
});

test('N2-2 the theatre bar steps on focus changes in a microtask, not inside an effect flush', () => {
  const s = read('../tabs/theatreBar.svelte.ts');
  assert.match(s, /const focus = \(\) => queueMicrotask\(\(\) => this\.step\(\)\);/);
});

test('N2-3 vote and bet labels keep a share of the row and the numbers wrap under them', () => {
  const s = read('./VotesBanner.svelte');
  assert.match(s, /\.row \{[^}]*flex-wrap: wrap;/s);
  assert.match(s, /\.label \{[^}]*min-width: 40%;[^}]*-webkit-line-clamp: 2;/s);
});

test('N2-4 the new/dropped pill stays on one line', () => {
  const s = read('./MessageList.svelte');
  assert.match(s, /\.more \{[^}]*white-space: nowrap;[^}]*overflow: hidden;/s);
  assert.match(s, /\.more-text \{[^}]*text-overflow: ellipsis;/s);
});

test('N2-8 the reopen list only scrolls when it is long', () => {
  const s = read('../tabs/ResumeDialog.svelte');
  assert.match(s, /class:scroll=\{list\.length > SCROLL_AFTER\}/);
  assert.doesNotMatch(s, /\.list \{[^}]*overflow-y/s);
  assert.match(s, /\.list\.scroll \{[^}]*overflow-y: auto;/s);
});
