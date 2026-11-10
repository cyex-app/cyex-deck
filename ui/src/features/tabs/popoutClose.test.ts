import assert from 'node:assert/strict';
import { test } from 'node:test';

import { popoutHint, popoutRefs } from './popoutClose.ts';

test('V110-A-idea tab pop-outs come first, then chat pop-outs', () => {
  const refs = popoutRefs({ tabs: new Set(['a', 'b']), chats: new Set(['t1']) });
  assert.deepEqual(refs, [
    { kind: 'tab', id: 'a' },
    { kind: 'tab', id: 'b' },
    { kind: 'chat', id: 't1' },
  ]);
  assert.deepEqual(popoutRefs({ tabs: new Set(), chats: new Set() }), []);
});

test('V110-A-idea the hint counts the windows', () => {
  assert.equal(popoutHint(0), 'No pop-outs open.');
  assert.equal(popoutHint(1), 'Puts 1 window back in the main window.');
  assert.equal(popoutHint(3), 'Puts 3 windows back in the main window.');
});
