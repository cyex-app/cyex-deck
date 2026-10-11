import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { PluginMenuEntry } from '../../ipc/bindings/PluginMenuEntry';
import { pluginItems } from './pluginMenus.ts';

const entry = (label: string, more: Partial<PluginMenuEntry> = {}): PluginMenuEntry => ({
  plugin: 'p',
  index: 0,
  label,
  place: 'message',
  group: null,
  lines: [],
  ...more,
});

test('plugin entries go to their menu, groups as submenus (S9)', () => {
  const menus = [
    entry('Translate', { lines: ['chat'] }),
    entry('Forecast', { place: 'chat', group: 'Weather' }),
    entry('Radar', { place: 'chat', group: 'Weather' }),
    entry('Pin it', { lines: ['notice'] }),
  ];
  const run = () => {};
  const labels = (list: ReturnType<typeof pluginItems>) => list.map((e) => (e === 'separator' ? '-' : e.label));
  assert.deepEqual(labels(pluginItems(menus, 'message', 'chat', run)), ['Translate']);
  assert.deepEqual(labels(pluginItems(menus, 'message', 'notice', run)), ['Pin it']);
  const chat = pluginItems(menus, 'chat', null, run);
  assert.deepEqual(labels(chat), ['Weather']);
  const sub = chat[0] !== 'separator' ? chat[0].items : [];
  assert.deepEqual(labels(sub ?? []), ['Forecast', 'Radar']);
});
