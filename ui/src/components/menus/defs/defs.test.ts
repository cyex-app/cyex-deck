import assert from 'node:assert/strict';
import { test } from 'node:test';

import { menuProblems } from '../../menuItems.ts';
import { idsOf, resolveMenu } from '../resolve.ts';
import { MENU_DEFS, MENUS, PREVIEW_AS, type PreviewAs } from './index.ts';

const top = (list: ReturnType<typeof resolveMenu>) => list.filter((e) => e !== 'separator').map((e) => (e as { label: string }).label);

const names = (id: string, who: PreviewAs, patch: object = {}) => {
  const m = MENUS.find((x) => x.def.id === id)!;
  return top(resolveMenu(m.def, undefined, { ...m.sample(who), ...patch }));
};

test('14 menus, each with a unique id', () => {
  assert.equal(MENU_DEFS.length, 14);
  assert.equal(new Set(MENU_DEFS.map((d) => d.id)).size, 14);
});

for (const { def, sample } of MENUS) {
  test(`${def.id}: item ids are unique and every default exists`, () => {
    const ids = def.items.map((i) => i.id);
    assert.equal(new Set(ids).size, ids.length);
    const groups = def.defaults.flatMap((n) => (n.t === 'group' ? [n.id] : []));
    for (const id of idsOf(def.defaults)) assert.ok(ids.includes(id) || groups.includes(id), `${def.id}: ${id}`);
  });

  for (const { id: who } of PREVIEW_AS) {
    test(`${def.id}: the built-in layout follows the menu rules as ${who}`, () => {
      const list = resolveMenu(def, undefined, sample(who));
      assert.deepEqual(menuProblems(list), []);
      assert.ok(list.length > 0);
    });
  }

  test(`${def.id}: every item builds with the samples`, () => {
    for (const { id: who } of PREVIEW_AS) {
      const ctx = sample(who);
      for (const i of def.items) assert.doesNotThrow(() => i.build(ctx), `${def.id}.${i.id}`);
    }
  });
}

test('chat.name: top level as in the plan', () => {
  assert.deepEqual(names('chat.name', 'viewer'), ['Usercard', 'Mention', 'Whisper', 'Copy name', 'Look up', 'Hide or block']);
  assert.deepEqual(names('chat.name', 'mod'), ['Usercard', 'Mention', 'Whisper', 'Copy name', 'Time out', 'Ban', 'Moderate', 'Look up', 'Hide or block']);
  assert.deepEqual(names('chat.name', 'out'), ['Usercard', 'Mention', 'Copy name', 'Look up', 'Hide or block']);
});

test('chat.message: top level as in the plan', () => {
  assert.deepEqual(names('chat.message', 'viewer'), ['Reply', 'Copy message', 'Translate', 'More']);
  assert.deepEqual(names('chat.message', 'mod'), ['Reply', 'Copy message', 'Translate', 'Time out', 'Delete message', 'Moderate', 'More']);
  // The broadcaster's line: Pin and Delete at the top.
  assert.deepEqual(names('chat.message', 'viewer', { pinLine: true, del: true }), [
    'Reply',
    'Copy message',
    'Translate',
    'Pin message',
    'Delete message',
    'More',
  ]);
});

test('the other menus keep their size', () => {
  assert.deepEqual(names('chat.emote', 'viewer'), ['Open emote card', 'Copy name', 'Hide Kappa', '7TV']);
  assert.deepEqual(names('chat.more', 'viewer'), [
    'Search',
    'Viewers',
    'Record chat',
    'Subscribe',
    'Overlay',
    'Translate messages',
    'Mute alerts',
    'Filters',
    'Layout',
    'Close',
  ]);
  assert.deepEqual(names('chat.more', 'out'), ['Search', 'Viewers', 'Record chat', 'Overlay', 'Translate messages', 'Mute alerts', 'Filters', 'Layout', 'Close']);
  assert.deepEqual(names('tabs.tab', 'viewer'), [
    'Edit',
    'Add a channel here',
    'Pop out tab',
    'Mute stream',
    'Stop playing',
    'View',
    'Group',
    'More',
    'Close',
    'Close more',
    'Reopen closed tab',
  ]);
  assert.deepEqual(names('tabs.tile', 'viewer'), ['Theatre', 'Sound here', 'Edit', 'Hide chat', 'Hide stream', 'Layout', 'Close']);
  assert.deepEqual(names('tabs.bar', 'viewer'), ['Add a tab', 'Only live tabs', 'Lock tabs', 'Tab bar position', 'Reopen closed tab']);
  assert.deepEqual(names('tabs.group', 'viewer'), ['Rename', 'Color', 'Ungroup', 'Close 3 tabs']);
  assert.deepEqual(names('player', 'viewer'), [
    'Pause',
    'Mute',
    'Quality',
    'Fullscreen',
    'Clip at current time',
    'Speed',
    'Picture in picture',
    'Moments',
    'Share',
    'Tools',
  ]);
  assert.deepEqual(names('browse.card', 'viewer'), ['Open', 'Open in the background', 'Open as', 'Favorite', 'Pick to watch side by side']);
  assert.deepEqual(names('feed.name', 'viewer'), ['Usercard', 'Whisper', 'Open this chat', 'Copy name']);
  assert.deepEqual(names('feed.message', 'viewer'), ['Open this chat', 'Show in logs', 'Copy message', 'Copy full message']);
  assert.deepEqual(names('whispers.thread', 'viewer'), ['Archive', 'Mute alerts', 'Block']);
  assert.deepEqual(names('clips.clip', 'viewer'), ['Play', 'Save as MP4', 'Copy link', 'Rename', 'Edit on Twitch', 'Delete clip']);
});

test('the new actions are in the palette, not in the built-in layouts', () => {
  const fresh: Record<string, string[]> = {
    'chat.name': ['openChat', 'watch', 'hideHere', 'copyId', 'shoutout', 'unblock'],
    'chat.message': ['copyId', 'copyLogsLink', 'mention', 'copyName', 'hideUser'],
    'chat.emote': ['favorite', 'copyLink'],
    'chat.more': ['popOut', 'copyLink', 'logs'],
    'tabs.tab': ['copyLink', 'theatre'],
    'browse.card': ['copyLink'],
    player: ['theatre'],
  };
  for (const [id, list] of Object.entries(fresh)) {
    const def = MENU_DEFS.find((d) => d.id === id)!;
    const inDefaults = idsOf(def.defaults);
    for (const n of list) {
      assert.ok(def.items.some((i) => i.id === n), `${id}.${n} exists`);
      assert.ok(!inDefaults.includes(n), `${id}.${n} stays out of the defaults`);
    }
  }
});

test('two-state labels can not be renamed', () => {
  for (const [id, item] of [
    ['chat.message', 'translate'],
    ['chat.name', 'mod'],
    ['tabs.tab', 'muteStream'],
    ['tabs.tab', 'popOut'],
    ['player', 'play'],
    ['player', 'fullscreen'],
    ['browse.card', 'favorite'],
    ['whispers.thread', 'archive'],
    ['clips.clip', 'save'],
  ] as const) {
    assert.equal(MENU_DEFS.find((d) => d.id === id)!.items.find((i) => i.id === item)!.renamable, false, `${id}.${item}`);
  }
});
