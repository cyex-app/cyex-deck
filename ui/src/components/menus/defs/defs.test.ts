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
  assert.deepEqual(names('chat.name', 'viewer'), ['Open usercard', 'Mention', 'Whisper', 'Copy name', 'Look up', 'Hide or block']);
  assert.deepEqual(names('chat.name', 'mod'), ['Open usercard', 'Mention', 'Whisper', 'Copy name', 'Look up', 'Hide or block', 'Moderator']);
  assert.deepEqual(names('chat.name', 'out'), ['Open usercard', 'Mention', 'Copy name', 'Look up', 'Hide or block']);
});

test('chat.message: top level as in the plan', () => {
  assert.deepEqual(names('chat.message', 'viewer'), ['Reply', 'Translate', 'Copy message', 'Copy with name and time', 'Look up']);
  assert.deepEqual(names('chat.message', 'mod'), ['Reply', 'Translate', 'Copy message', 'Copy with name and time', 'Look up', 'Moderator']);
  // The broadcaster's line: Pin and Delete are in the Moderator group.
  assert.deepEqual(names('chat.message', 'viewer', { pinLine: true, del: true }), [
    'Reply',
    'Translate',
    'Copy message',
    'Copy with name and time',
    'Look up',
    'Moderator',
  ]);
});

test('chat.message: a deleted line offers no Reply or Translate (V110-F93)', () => {
  assert.deepEqual(names('chat.message', 'viewer', { deleted: true }), ['Copy message', 'Copy with name and time', 'Look up']);
});

test('the other menus keep their size', () => {
  assert.deepEqual(names('chat.emote', 'viewer'), ['Open emote card', 'Copy emote name', 'Hide Kappa', '7TV']);
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
  assert.deepEqual(names('tabs.bar', 'viewer'), ['Add a tab', 'Only live tabs', 'Lock tabs', 'Tab bar position', 'Reopen closed tab', 'Close all pop-outs']);
  assert.deepEqual(names('tabs.group', 'viewer'), ['Rename', 'Color', 'Ungroup', 'Close 3 tabs']);
  assert.deepEqual(names('player', 'viewer'), [
    'Pause',
    'Mute',
    'Quality',
    'Fullscreen',
    'Clip at current time',
    'Clip',
    'Speed',
    'Picture in picture',
    'Moments',
    'Pop out chat',
    'Share',
    'Tools',
  ]);
  assert.deepEqual(names('browse.card', 'viewer'), ['Open', 'Open in the background', 'Open as', 'Favorite', 'Pick to watch side by side']);
  assert.deepEqual(names('feed.name', 'viewer'), ['Open usercard', 'Whisper', 'Open this chat', 'Copy name']);
  assert.deepEqual(names('feed.message', 'viewer'), ['Open this chat', 'Show message in logs', 'Copy message', 'Copy with name and time']);
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

test('V110-F79 a group of one tab closes "1 tab"', async () => {
  const { closeTabs } = await import('./tabbar.ts');
  assert.equal(closeTabs(1), 'Close 1 tab');
  assert.equal(closeTabs(3), 'Close 3 tabs');
});

const MOD_LABELS = ['Time out user', 'Ban user', 'Warn user', 'Unban user', 'Monitor user', 'Restrict user', 'Delete message', 'Pin message', 'Shout out user', 'Moderator'];
const flat = (list: ReturnType<typeof resolveMenu>): string[] =>
  list.flatMap((e) => (e === 'separator' ? [] : [e.label, ...(e.items ? flat(e.items) : [])]));

test('V110-B8 no moderator action in a name or message menu for an account that does not moderate', () => {
  for (const id of ['chat.name', 'chat.message']) {
    const m = MENUS.find((x) => x.def.id === id)!;
    for (const who of ['viewer', 'out'] as const) {
      const all = flat(resolveMenu(m.def, undefined, m.sample(who)));
      for (const l of MOD_LABELS) assert.ok(!all.includes(l), `${id} as ${who} shows ${l}`);
      // Nor does a layout that puts the mod rows at the top level.
      const nodes = m.def.items.filter((i) => i.mod).map((i) => ({ t: 'item' as const, id: i.id, label: null, hidden: false }));
      const flatLayout = flat(resolveMenu(m.def, { nodes, known: m.def.items.map((i) => i.id) }, m.sample(who)));
      assert.deepEqual(flatLayout, [], `${id} as ${who} with a mod-only layout`);
    }
    assert.ok(flat(resolveMenu(m.def, undefined, m.sample('mod'))).includes('Ban user'), `${id} as a mod shows Ban user`);
  }
});

test('V110-B8 every row marked "mods" is a moderator row, and no mod row builds for a viewer', () => {
  for (const { def, sample } of MENUS) {
    for (const i of def.items) {
      if (i.when === 'mods') assert.equal(i.mod, true, `${def.id}.${i.id} is mod-only`);
      if (!i.mod) continue;
      for (const who of ['viewer', 'out'] as const) assert.equal(i.build(sample(who)), null, `${def.id}.${i.id} as ${who}`);
    }
  }
});

test('V110-B8 the Moderator group holds all and only the mod rows of the built-in layout', () => {
  for (const id of ['chat.name', 'chat.message']) {
    const def = MENU_DEFS.find((d) => d.id === id)!;
    const modIds = def.items.filter((i) => i.mod).map((i) => i.id);
    const g = def.defaults.find((n) => n.t === 'group' && n.id === 'moderate');
    assert.equal(g?.t === 'group' && g.label, 'Moderator');
    const inGroup = idsOf(g && g.t === 'group' ? g.children : []);
    const outside = idsOf(def.defaults.filter((n) => n !== g)).filter((x) => modIds.includes(x));
    assert.deepEqual(outside, [], `${id}: mod rows outside the group`);
    for (const x of inGroup) assert.ok(modIds.includes(x), `${id}.${x} is a viewer row in the Moderator group`);
  }
});

test('V110-B8 two rows of one menu never share a name', () => {
  for (const { def } of MENUS) {
    const names = def.items.filter((i) => !i.slot).map((i) => i.name);
    const dup = names.filter((n, k) => names.indexOf(n) !== k);
    assert.deepEqual(dup, [], `${def.id}`);
  }
});
