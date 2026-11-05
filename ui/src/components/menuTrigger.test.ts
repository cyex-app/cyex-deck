import assert from 'node:assert/strict';
import { test } from 'node:test';

import { menuTrigger, type TriggerEl } from './menuTrigger.ts';

function button(inner: unknown[] = []): TriggerEl & { attrs: Map<string, string> } {
  const attrs = new Map<string, string>();
  const self: TriggerEl & { attrs: Map<string, string> } = {
    attrs,
    isConnected: true,
    contains: (n) => n === self || inner.includes(n),
    getAttribute: (k) => attrs.get(k) ?? null,
    setAttribute: (k, v) => void attrs.set(k, v),
    removeAttribute: (k) => void attrs.delete(k),
  };
  return self;
}

test('the button of the open menu is recognised, others are not', () => {
  const t = menuTrigger();
  const a = button();
  const b = button();
  t.set(a);
  assert.equal(t.isOpenFor(a), true);
  assert.equal(t.isOpenFor(b), false);
});

test('a click inside the button (its icon) counts as the button for outside clicks', () => {
  const icon = {};
  const t = menuTrigger();
  t.set(button([icon]));
  assert.equal(t.owns(icon), true);
  assert.equal(t.owns({}), false);
});

test('aria-expanded follows the menu: set on open, gone on close', () => {
  const t = menuTrigger();
  const a = button();
  t.set(a);
  assert.equal(a.attrs.get('aria-expanded'), 'true');
  t.clear();
  assert.equal(a.attrs.has('aria-expanded'), false);
  assert.equal(t.isOpenFor(a), false);
});

test('moving to another button lets the first one go', () => {
  const t = menuTrigger();
  const a = button();
  const b = button();
  t.set(a);
  t.set(b);
  assert.equal(a.attrs.has('aria-expanded'), false);
  assert.equal(b.attrs.get('aria-expanded'), 'true');
  assert.equal(t.isOpenFor(a), false);
});

test('a menu at the pointer has no button', () => {
  const t = menuTrigger();
  const a = button();
  t.set(a);
  t.set(null);
  assert.equal(a.attrs.has('aria-expanded'), false);
  assert.equal(t.owns(a), false);
  assert.equal(t.isOpenFor(a), false);
});

test('a button that already had aria-expanded gets it back on close', () => {
  const t = menuTrigger();
  const a = button();
  a.attrs.set('aria-expanded', 'false');
  t.set(a);
  assert.equal(a.attrs.get('aria-expanded'), 'true');
  t.clear();
  assert.equal(a.attrs.get('aria-expanded'), 'false');
});

test('a button that left the page no longer counts', () => {
  const t = menuTrigger();
  const a = button();
  t.set(a);
  a.isConnected = false;
  assert.equal(t.isOpenFor(a), false);
  assert.equal(t.owns(a), false);
});
