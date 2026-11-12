import assert from 'node:assert/strict';
import { test } from 'node:test';

import { PressGate, ReloadRegistry, reloadPicks } from './reload.ts';

const item = (tile?: string) => ({ tile, run() {} });
const tabs = [
  { id: 'a', tiles: [{ id: 'a1' }, { id: 'a2' }] },
  { id: 'b', tiles: [{ id: 'b1' }] },
  { id: 'f', tiles: [{ id: 'f1', feed: {} }] },
];
const chats = [item('a1'), item('a2'), item('b1'), item('f1'), item(undefined)];
const players = [item('a1'), item('b1')];

test('V110-N7 one tab reloads only its own chats and players', () => {
  const r = reloadPicks({ scope: 'tab', shown: tabs[0], tabs, chats, players });
  assert.deepEqual(r.chats.map((x) => x.tile), ['a1', 'a2']);
  assert.deepEqual(r.players.map((x) => x.tile), ['a1']);
});

test('V110-N7 a feed tab, or no shown tab, reloads nothing', () => {
  const feed = reloadPicks({ scope: 'tab', shown: tabs[2], tabs, chats, players });
  assert.equal(feed.chats.length + feed.players.length, 0);
  const none = reloadPicks({ scope: 'tab', tabs, chats, players });
  assert.equal(none.chats.length + none.players.length, 0);
});

test('V110-N7 all tabs reloads every tile with a chat or player, never one without a tile', () => {
  const r = reloadPicks({ scope: 'all', tabs, chats, players });
  assert.deepEqual(r.chats.map((x) => x.tile), ['a1', 'a2', 'b1']);
  assert.deepEqual(r.players.map((x) => x.tile), ['a1', 'b1']);
});

test('V110-N7 a second press within 2 s is ignored', () => {
  const g = new PressGate(2000);
  assert.ok(g.allow(10_000));
  assert.ok(!g.allow(11_999));
  assert.ok(g.allow(12_000));
});

test('V110-N7 the registry forgets what unregisters', () => {
  const r = new ReloadRegistry();
  const off = r.add(item('a1'));
  assert.equal(r.list().length, 1);
  off();
  assert.equal(r.list().length, 0);
});

test('V110-N7 Ctrl+R tells pop-outs to reload: main emits, panels listen once', async () => {
  const { readFileSync } = await import('node:fs');
  const read = (p: string) => readFileSync(new URL(p, import.meta.url), 'utf8');
  const { RELOAD_ALL_EVENT } = await import('./reload.ts');
  assert.match(RELOAD_ALL_EVENT, /^[a-z0-9:_/-]+$/i);
  assert.match(read('../../app/App.svelte'), /emit\(RELOAD_ALL_EVENT\)/);
  assert.match(read('../../app/PanelApp.svelte'), /listen\(RELOAD_ALL_EVENT, \(\) => reloadEverything\(\)\)/);
});
