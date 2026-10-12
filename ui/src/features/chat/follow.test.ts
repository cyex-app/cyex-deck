import assert from 'node:assert/strict';
import { test } from 'node:test';

import { elsewhere, settled } from './follow.ts';

test('V103-22 a window decides only once no other window is still loading', () => {
  assert.equal(settled({ chats: {}, loading: [] }, 'panel-modview_xqc'), true);
  // Its own loading doesn't hold it up; another window's does.
  assert.equal(settled({ chats: {}, loading: ['main'] }, 'main'), true);
  assert.equal(settled({ chats: {}, loading: ['main', 'panel-tab_a'] }, 'main'), false);
  assert.equal(settled({ chats: {}, loading: ['panel-tab_a'] }, 'panel-modview_xqc'), false);
});

test('V103-22 a regular chat counts only in another window', () => {
  const chats = { xqc: ['main', 'panel-chat_t1'], jynxzi: ['panel-modview_jynxzi'] };
  assert.equal(elsewhere(chats, 'xqc', 'panel-modview_xqc'), true);
  assert.equal(elsewhere(chats, '#XQC', 'main'), true, 'the popped-out chat');
  assert.equal(elsewhere(chats, 'jynxzi', 'panel-modview_jynxzi'), false, 'its own window');
  assert.equal(elsewhere(chats, 'papaplatte', 'main'), false);
});
