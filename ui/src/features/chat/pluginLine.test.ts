import assert from 'node:assert/strict';
import { test } from 'node:test';

import { actTip, picSize, spanColor } from './pluginLine.ts';

test('a clickable piece says what it does (S7)', () => {
  assert.equal(actTip({ kind: 'run', text: '/weather vote' }), 'Runs /weather vote');
  assert.equal(actTip({ kind: 'run', text: 'hi all' }), 'Sends hi all');
  assert.equal(actTip({ kind: 'copy', text: 'x'.repeat(70) }), `Copies ${'x'.repeat(60)}…`);
  assert.equal(actTip({ kind: 'url', url: 'https://a.com', host: 'a.com', target: null }), 'a.com');
});

test('span colors and picture sizes (S7, S10)', () => {
  assert.deepEqual(spanColor(undefined), { cls: '', vars: '' });
  assert.deepEqual(spanColor('link'), { cls: 'c-link', vars: '' });
  assert.deepEqual(spanColor({ rgb: { dark: '#0f0', light: '#060' } }), { cls: 'c-rgb', vars: '--cd:#0f0;--cl:#060' });
  assert.deepEqual(picSize([28, 28]), { width: 28, height: 28 });
  assert.deepEqual(picSize([224, 224]), { width: 112, height: 112 });
  assert.deepEqual(picSize([1000, 50]), { width: 448, height: 22 });
  assert.equal(picSize(undefined), null);
});
