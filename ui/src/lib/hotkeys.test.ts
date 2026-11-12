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

test('V110-F81 held toggles run once; seek, volume and zoom repeat', async () => {
  const { repeatAllowed } = await import('./hotkeyActions.ts');
  for (const id of ['player.playPause', 'player.mute', 'player.fullscreen', 'tabs.new', 'tabs.close', 'settings.open', 'player.live']) {
    assert.equal(repeatAllowed(id, true), false, id);
    assert.equal(repeatAllowed(id, false), true, id);
  }
  for (const id of ['player.back', 'player.forward2', 'player.volumeUp', 'chat.lineDown', 'zoom.in']) {
    assert.equal(repeatAllowed(id, true), true, id);
  }
});

test('V110-F81 every repeatable id is a real action', async () => {
  const { repeatAllowed, ACTIONS } = await import('./hotkeyActions.ts');
  const repeating = Object.keys(ACTIONS).filter((id) => repeatAllowed(id, true));
  assert.equal(repeating.length, 14);
});

test('V110-F81 AltGr is typing', async () => {
  const { typesChar } = await import('./hotkeyActions.ts');
  assert.equal(typesChar({ getModifierState: (k) => k === 'AltGraph' }), true);
  assert.equal(typesChar({ getModifierState: () => false }), false);
  assert.equal(typesChar({}), false);
});

test('V110-F81 key labels read as words and arrows', async () => {
  const { keyLabel } = await import('./hotkeyActions.ts');
  assert.deepEqual('Shift+ArrowRight'.split('+').map(keyLabel), ['Shift', '→']);
  assert.equal(keyLabel('PageUp'), 'Page up');
  assert.equal(keyLabel('PageDown'), 'Page down');
  assert.equal(keyLabel('Escape'), 'Esc');
  assert.equal(keyLabel('Space'), 'Space');
  assert.equal(keyLabel('Backspace'), 'Backspace');
  assert.equal(keyLabel('Ctrl'), 'Ctrl');
});

test('V110-N4 Ctrl+F5 reloads the app, also over the settings sheet', async () => {
  const { OVER_SHEET, HOTKEY_GROUPS, hotkeyGroup } = await import('./hotkeyActions.ts');
  assert.equal(ACTIONS['app.reload'].default, 'Ctrl+F5');
  assert.equal(resolveBinding('app.reload'), 'Ctrl+F5');
  assert.ok(OVER_SHEET.has('app.reload'));
  for (const id of Object.keys(ACTIONS)) assert.ok(HOTKEY_GROUPS.includes(hotkeyGroup(id)), id);
  assert.equal(hotkeyGroup('app.reload'), 'View');
  assert.equal(hotkeyGroup('settings.open'), 'Navigation');
});

test('V110-N7 F5 reloads the tab, Ctrl+R all tabs, both under Tabs and kept from the page', async () => {
  const { OVER_SHEET, hotkeyGroup } = await import('./hotkeyActions.ts');
  assert.equal(resolveBinding('tab.reload'), 'F5');
  assert.equal(resolveBinding('tabs.reloadAll'), 'Ctrl+R');
  assert.equal(ACTIONS['tab.reload'].label, 'Reload this tab');
  assert.equal(ACTIONS['tabs.reloadAll'].label, 'Reload all tabs');
  assert.equal(hotkeyGroup('tab.reload'), 'Tabs');
  assert.equal(hotkeyGroup('tabs.reloadAll'), 'Tabs');
  assert.ok(OVER_SHEET.has('tab.reload') && OVER_SHEET.has('tabs.reloadAll'));
});
