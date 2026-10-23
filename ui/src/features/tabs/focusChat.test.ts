import assert from 'node:assert/strict';
import { test } from 'node:test';

import { inOtherField, mayFocusChat } from './focusChat.ts';

/** Just enough of an element for the rules. */
function el(opts: { tag?: string; type?: string; field?: boolean; chat?: boolean; switcher?: boolean }): Element {
  return {
    tagName: opts.tag ?? 'INPUT',
    type: opts.type,
    matches: (sel: string) => (sel.includes('.compose') ? !!opts.chat : sel.includes('input') ? opts.field !== false : false),
    closest: (sel: string) => (sel.includes('Quick switcher') && opts.switcher ? {} : null),
  } as unknown as Element;
}

test('focus follows a switch unless a layer is open or another field has it (V107-H15)', () => {
  assert.equal(mayFocusChat({ otherField: false, layerOpen: false }), true);
  assert.equal(mayFocusChat({ otherField: false, layerOpen: true }), false);
  assert.equal(mayFocusChat({ otherField: true, layerOpen: false }), false);
});

test('which focused elements count as typing elsewhere', () => {
  assert.equal(inOtherField(null), false);
  assert.equal(inOtherField(el({ field: false, tag: 'BUTTON' })), false, 'a tab or button');
  assert.equal(inOtherField(el({})), true, 'a text box');
  assert.equal(inOtherField(el({ type: 'checkbox' })), false);
  assert.equal(inOtherField(el({ tag: 'TEXTAREA', chat: true })), false, 'a chat input hands over');
  assert.equal(inOtherField(el({ switcher: true })), false, 'the switcher\'s own search');
});
