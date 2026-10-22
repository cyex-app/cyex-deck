import assert from 'node:assert/strict';
import { test } from 'node:test';

import { ACTIONS, comboAlias, resolveBinding } from './hotkeyActions.ts';

test('every default combo is used once', () => {
  const seen = new Set<string>();
  for (const [id, a] of Object.entries(ACTIONS)) {
    if (!a.default) continue;
    assert.ok(!seen.has(a.default), `${id} repeats ${a.default}`);
    seen.add(a.default);
  }
});

test('only live tabs is on Ctrl+Shift+L (V106-06)', () => {
  assert.equal(ACTIONS['tabs.onlyLive'].default, 'Ctrl+Shift+L');
});

test('a default someone else was rebound to stays theirs (V106-06)', () => {
  assert.equal(resolveBinding('tabs.onlyLive', {}), 'Ctrl+Shift+L');
  assert.equal(resolveBinding('tabs.onlyLive', { 'tabs.new': 'Ctrl+Shift+L' }), '');
  assert.equal(resolveBinding('tabs.new', { 'tabs.new': 'Ctrl+Shift+L' }), 'Ctrl+Shift+L');
  assert.equal(resolveBinding('tabs.onlyLive', { 'tabs.onlyLive': 'Ctrl+L', 'tabs.new': 'Ctrl+Shift+L' }), 'Ctrl+L');
});

test('an override wins, empty unbinds, a reset goes back to the default', () => {
  assert.equal(resolveBinding('tabs.new', { 'tabs.new': '' }), '');
  assert.equal(resolveBinding('tabs.new', { 'tabs.new': null }), 'Ctrl+T');
  assert.equal(resolveBinding('global.show', {}), '');
});

test('Ctrl++ from the numpad or a German "+" key zooms in like Ctrl+= (V107-N6a)', () => {
  assert.equal(comboAlias('Ctrl++'), 'Ctrl+=');
  assert.equal(comboAlias('Ctrl+Shift++'), 'Ctrl+=');
  assert.equal(comboAlias('Ctrl+-'), undefined);
});
