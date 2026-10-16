import assert from 'node:assert/strict';
import { test } from 'node:test';

import { type EscapeEvent, type LayerKind, LayerStack } from './escape.ts';

function key(k: string, target: EscapeEvent['target'] = null) {
  const e = {
    key: k,
    defaultPrevented: false,
    target,
    stopped: false,
    preventDefault() {
      e.defaultPrevented = true;
    },
    stopImmediatePropagation() {
      e.stopped = true;
    },
  };
  return e;
}

/** A layer whose owner takes it off the stack when it closes, as the components do. */
function open(s: LayerStack, kind: LayerKind, closed: string[], name: string) {
  const off = s.push({
    kind,
    close: () => {
      closed.push(name);
      off();
    },
  });
  return off;
}

test('Escape closes only the topmost layer, then the next', () => {
  const s = new LayerStack();
  const closed: string[] = [];
  open(s, 'dialog', closed, 'dialog');
  open(s, 'popover', closed, 'popover');
  const e = key('Escape');
  assert.equal(s.escape(e), true);
  assert.deepEqual(closed, ['popover']);
  assert.equal(e.defaultPrevented, true);
  assert.equal(e.stopped, true);
  assert.equal(s.escape(key('Escape')), true);
  assert.deepEqual(closed, ['popover', 'dialog']);
  assert.equal(s.escape(key('Escape')), false);
});

test('a layer that refuses to close keeps holding Escape', () => {
  const s = new LayerStack();
  let tries = 0;
  s.push({ kind: 'dialog', close: () => tries++ });
  assert.equal(s.escape(key('Escape')), true);
  assert.equal(s.escape(key('Escape')), true);
  assert.equal(tries, 2);
  assert.equal(s.size, 1);
});

test('other keys, used keys and an empty stack are left alone', () => {
  const s = new LayerStack();
  assert.equal(s.escape(key('Escape')), false);
  s.push({ kind: 'menu', close: () => assert.fail() });
  assert.equal(s.escape(key('Enter')), false);
  const used = key('Escape');
  used.defaultPrevented = true;
  assert.equal(s.escape(used), false);
});

test('a field with data-own-escape keeps its Escape', () => {
  const s = new LayerStack();
  s.push({ kind: 'dialog', close: () => assert.fail() });
  const field = { closest: (sel: string) => (sel === '[data-own-escape]' ? field : null) };
  const e = key('Escape', field as unknown as EventTarget);
  assert.equal(s.escape(e), false);
  assert.equal(e.defaultPrevented, false);
});

test('removing is safe twice and keeps the others in order', () => {
  const s = new LayerStack();
  const closed: string[] = [];
  const offA = open(s, 'dialog', closed, 'a');
  open(s, 'popover', closed, 'b');
  offA();
  offA();
  assert.equal(s.size, 1);
  s.escape(key('Escape'));
  assert.deepEqual(closed, ['b']);
  assert.equal(s.size, 0);
});

test('only dialogs, menus and open selects block shortcuts', () => {
  const s = new LayerStack();
  const off = s.push({ kind: 'popover', close: () => {} });
  assert.equal(s.blocking(), false);
  const offMenu = s.push({ kind: 'menu', close: () => {} });
  assert.equal(s.blocking(), true);
  offMenu();
  off();
  assert.equal(s.blocking(), false);
});
