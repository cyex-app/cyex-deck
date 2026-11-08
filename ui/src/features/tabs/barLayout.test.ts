import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Tab } from '../../ipc/bindings/Tab';
import type { Tile } from '../../ipc/bindings/Tile';
import { ALL_KEEP, barMaxHeight, barPlace, type ChatBeside, chatBeside, hiddenTabs, NARROW, plusFits, sideBarWidth } from './barLayout.ts';

function tile(channel: string, feed: string | null = null): Tile {
  return {
    id: `t-${channel}`,
    channel: { id: channel, login: channel, name: channel },
    show: { player: true, chat: true },
    account: null,
    ratio: 0.28,
    size: 1,
    volume: 1,
    muted: false,
    boost: false,
    feed,
    feedKind: 'rules',
    filters: [],
    chatSide: null,
    modviewShare: null,
  };
}

function tab(id: string, ...tiles: Tile[]): Tab {
  return { id, alias: null, group: null, columns: [{ size: 1, tiles }] };
}

test('F03 top tabs leave the title bar below 900 px', () => {
  assert.equal(barPlace('top', 1280), 'chrome');
  assert.equal(barPlace('top', NARROW), 'chrome', '900 itself is wide');
  assert.equal(barPlace('top', NARROW - 1), 'row');
  assert.equal(barPlace('top', 720), 'row');
  assert.equal(barPlace('top', 0), 'chrome', 'not measured yet');
  assert.equal(barPlace('bottom', 720), 'bottom', 'other sides never move');
  assert.equal(barPlace('left', 720), 'left');
});

test('F16 whole rows, or all but the bottom of the window', () => {
  assert.equal(barMaxHeight('limit', 3, 34), '102px');
  assert.equal(barMaxHeight('limit', 1, 29), '29px');
  assert.equal(barMaxHeight('limit', 0, 34), '34px', 'at least one row');
  assert.equal(barMaxHeight('all', 3, 34), `calc(100vh - ${ALL_KEEP}px)`);
});

test('F17 only live tabs', () => {
  const live = {
    a: { stream: { rerun: false } },
    r: { stream: { rerun: true } },
    off: { stream: null },
    off2: { stream: null },
  };
  const list = [
    tab('live', tile('a')),
    tab('rerun', tile('r')),
    tab('offline', tile('off')),
    tab('mixed', tile('off'), tile('a')),
    tab('all-off', tile('off'), tile('off2')),
    tab('unchecked', tile('new')),
    tab('half-checked', tile('off'), tile('new')),
    tab('feed', tile('', 'Mentions')),
  ];
  assert.deepEqual([...hiddenTabs(list, false, [], live)], [], 'off: nothing hidden');
  // A channel the check didn't find (banned, deleted) counts as offline.
  assert.deepEqual([...hiddenTabs(list, true, [null], live)], ['offline', 'all-off', 'unchecked', 'half-checked']);
  // The tab on screen, one playing out of sight and popped-out ones stay.
  assert.deepEqual([...hiddenTabs(list, true, ['offline', 'all-off', 'unchecked', 'half-checked'], live)], []);
  // Before the first live check nothing hides.
  assert.deepEqual([...hiddenTabs(list, true, [], {})], []);
});

test('a left bar ends on the header divider (V106-26)', () => {
  assert.equal(sideBarWidth('left', 180), 179);
  assert.equal(sideBarWidth('left', 0), undefined);
  assert.equal(sideBarWidth('right', 180), undefined);
  assert.equal(sideBarWidth('top', 180), undefined);
  assert.equal(sideBarWidth('bottom', 180), undefined);
});

test('V108-F59d plusFits: the tab and the plus share the view or not', () => {
  assert.equal(plusFits(100, 190, 100), true);
  assert.equal(plusFits(100, 201, 100), false);
  assert.equal(plusFits(100, 90, 100), false);
});

test('V110-U52 above chat: only where the chat is beside the stream', () => {
  assert.equal(barPlace('aboveChat', 1280, true), 'above');
  assert.equal(barPlace('aboveChat', 1280, false), 'chrome', 'falls back to the top');
  assert.equal(barPlace('aboveChat', 720, false), 'row', 'and the narrow-window row');
  assert.equal(barPlace('aboveChat', 720, true), 'above');
  assert.equal(barPlace('top', 1280, true), 'chrome', 'other sides ignore it');
  assert.equal(barPlace('left', 1280, true), 'left');
  const ok: ChatBeside = { tiles: 1, feed: false, stream: true, chat: true, chatPopped: false, side: 'right' };
  assert.equal(chatBeside(ok), true);
  assert.equal(chatBeside({ ...ok, side: 'left' }), true);
  assert.equal(chatBeside({ ...ok, side: 'bottom' }), false, 'chat under');
  assert.equal(chatBeside({ ...ok, side: 'top' }), false, 'chat over');
  assert.equal(chatBeside({ ...ok, chat: false }), false, 'no chat');
  assert.equal(chatBeside({ ...ok, stream: false }), false, 'chat only');
  assert.equal(chatBeside({ ...ok, chatPopped: true }), false, 'chat in its own window');
  assert.equal(chatBeside({ ...ok, feed: true }), false);
  assert.equal(chatBeside({ ...ok, tiles: 2 }), false, 'several tiles');
});
