import assert from 'node:assert/strict';
import { test } from 'node:test';

import { menuProblems } from '../../menuItems.ts';
import { idsOf, resolveMenu } from '../resolve.ts';
import { MENU_DEFS, MENUS, MENUS_BY_GROUP, PREVIEW_AS, type PreviewAs } from './index.ts';

const top = (list: ReturnType<typeof resolveMenu>) => list.filter((e) => e !== 'separator').map((e) => (e as { label: string }).label);

const names = (id: string, who: PreviewAs, patch: object = {}) => {
  const m = MENUS.find((x) => x.def.id === id)!;
  return top(resolveMenu(m.def, undefined, { ...m.sample(who), ...patch }));
};

test('15 menus, each with a unique id', () => {
  assert.equal(MENU_DEFS.length, 15);
  assert.equal(new Set(MENU_DEFS.map((d) => d.id)).size, 15);
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
  const base = ['Open usercard', 'Mention', 'Whisper', 'Copy name', 'Search their messages', 'Open their logs', 'Hide user for me', 'Block on Twitch'];
  assert.deepEqual(names('chat.name', 'viewer'), base);
  assert.deepEqual(names('chat.name', 'mod'), [...base, 'Time out user', 'Moderator']);
  assert.deepEqual(names('chat.name', 'out'), ['Open usercard', 'Mention', 'Copy name', 'Search their messages', 'Open their logs', 'Hide user for me']);
});

test('chat.message: top level as in the plan', () => {
  assert.deepEqual(names('chat.message', 'viewer'), ['Reply', 'Translate', 'Copy message', 'Copy full message', 'Look up']);
  assert.deepEqual(names('chat.message', 'mod'), ['Reply', 'Translate', 'Copy message', 'Copy full message', 'Look up', 'Time out user', 'Delete message', 'Moderator']);
  // The broadcaster's line: the one Pin message row shows (alone, so not in a submenu); Delete stays on top.
  assert.deepEqual(names('chat.message', 'viewer', { pinLine: true, del: true }), [
    'Reply',
    'Translate',
    'Copy message',
    'Copy full message',
    'Look up',
    'Delete message',
    'Pin message',
  ]);
});

test('chat.message: a deleted line offers no Reply or Translate (V110-F93)', () => {
  assert.deepEqual(names('chat.message', 'viewer', { deleted: true }), ['Copy message', 'Copy full message', 'Look up']);
});

test('the other menus keep their size', () => {
  assert.deepEqual(names('chat.emote', 'viewer'), ['Open emote card', 'Copy emote name', 'Hide emote', 'Add to 7TV channel']);
  assert.deepEqual(names('chat.more', 'viewer'), [
    'Search',
    'Viewers',
    'Translate messages',
    'Mute alerts',
    'Filters',
    'Record chat',
    'Overlay',
    'Pop out chat',
    'Tab',
    'Subscribe',
    'Close',
  ]);
  assert.deepEqual(names('chat.more', 'out'), [
    'Search',
    'Viewers',
    'Translate messages',
    'Mute alerts',
    'Filters',
    'Record chat',
    'Overlay',
    'Pop out chat',
    'Tab',
    'Close',
  ]);
  assert.deepEqual(names('tabs.tab', 'viewer'), [
    'Edit',
    'Add a channel here',
    'Pop out tab',
    'Mute stream',
    'Stop playing',
    'Mute alerts',
    'Mine first',
    'View',
    'Group',
    'Close',
    'Close other tabs',
    'Close tabs after this',
    'Reopen closed tab',
  ]);
  assert.deepEqual(names('tabs.tile', 'viewer'), ['Theatre', 'Sound here', 'Edit', 'Move to its own tab', 'View', 'Close']);
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
  assert.deepEqual(names('feed.name', 'viewer'), ['Open usercard', 'Whisper', 'Open chat', 'Copy name']);
  assert.deepEqual(names('feed.message', 'viewer'), ['Copy message', 'Copy full message', 'Open chat', 'Show in chat', 'Show message in logs']);
  assert.deepEqual(names('whispers.thread', 'viewer'), ['Archive', 'Mute alerts', 'Block on Twitch']);
  assert.deepEqual(names('clips.clip', 'viewer'), ['Play', 'Save as MP4', 'Copy link', 'Rename', 'Edit on Twitch', 'Delete clip']);
});

test('the new actions are in the palette, not in the built-in layouts', () => {
  const fresh: Record<string, string[]> = {
    'chat.name': ['openChat', 'watch', 'hideHere', 'copyId', 'shoutout', 'unblock'],
    'chat.message': ['copyId', 'copyLogsLink', 'mention', 'copyName', 'hideUser'],
    'chat.emote': ['favorite', 'copyLink'],
    'chat.more': ['copyLink', 'logs'],
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

// Time out (names, messages) and Delete (messages) stay at the top level (still mod-only rows).
const TOP_MOD: Record<string, string[]> = { 'chat.name': ['timeout'], 'chat.message': ['timeout', 'delete'] };

test('V110-B8 the Moderator group holds all and only the mod rows of the built-in layout', () => {
  for (const id of ['chat.name', 'chat.message']) {
    const def = MENU_DEFS.find((d) => d.id === id)!;
    const modIds = def.items.filter((i) => i.mod).map((i) => i.id);
    const g = def.defaults.find((n) => n.t === 'group' && n.id === 'moderate');
    assert.equal(g?.t === 'group' && g.label, 'Moderator');
    const inGroup = idsOf(g && g.t === 'group' ? g.children : []);
    const outside = idsOf(def.defaults.filter((n) => n !== g))
      .filter((x) => modIds.includes(x))
      .filter((x) => !(TOP_MOD[id] ?? []).includes(x));
    assert.deepEqual(outside, [], `${id}: mod rows outside the group`);
    for (const x of inGroup) assert.ok(modIds.includes(x), `${id}.${x} is a viewer row in the Moderator group`);
  }
});

test('V111-10 the same action has the same words in every menu', () => {
  const labels = (id: string) => MENU_DEFS.find((d) => d.id === id)!.items.map((i) => i.name);
  assert.ok(labels('chat.name').includes('Open chat') && labels('feed.name').includes('Open chat') && labels('feed.message').includes('Open chat'));
  assert.ok(labels('whispers.thread').includes('Block on Twitch') && labels('chat.name').includes('Block on Twitch'));
  for (const id of ['chat.more', 'tabs.tab', 'browse.card']) assert.ok(labels(id).includes('Copy link'), id);
  const m = MENUS.find((x) => x.def.id === 'chat.message')!;
  assert.ok(!m.def.items.some((i) => i.id === 'pinLine'));
});

test('V111-10 Tab group of the chat header: only in a tab, rows of the tile menu', () => {
  const m = MENUS.find((x) => x.def.id === 'chat.more')!;
  const withTab = resolveMenu(m.def, undefined, m.sample('viewer')).find((e) => e !== 'separator' && e.label === 'Tab');
  assert.deepEqual(withTab && withTab !== 'separator' && flat(withTab.items ?? []), ['Theatre', 'Sound here', 'Edit', 'Move to its own tab', 'Hide chat', 'Hide stream', 'Move panels', 'Reset layout', 'Chat look', 'Text size']);
  const none = top(resolveMenu(m.def, undefined, { ...m.sample('viewer'), tile: null }));
  assert.ok(!none.includes('Tab') && !none.includes('Close'));
});

test('V110-B8 two rows of one menu never share a name', () => {
  for (const { def } of MENUS) {
    const names = def.items.filter((i) => !i.slot).map((i) => i.name);
    const dup = names.filter((n, k) => names.indexOf(n) !== k);
    assert.deepEqual(dup, [], `${def.id}`);
  }
});

test('V110-N9 the manager groups the 15 menus under four headings with short names', () => {
  assert.deepEqual(
    MENUS_BY_GROUP.map((g) => [g.group, g.menus.map((m) => m.short)]),
    [
      ['Chat', ['Names', 'Messages', 'Emotes', 'Chat header', 'Chat modes', 'Whispers']],
      ['Feeds', ['Names', 'Messages']],
      ['Tabs', ['Tabs', 'Tab groups', 'Tab bar', 'Tiles']],
      ['Watching', ['Players', 'Channel cards', 'Clips']],
    ],
  );
  assert.equal(MENUS_BY_GROUP.flatMap((g) => g.menus).length, MENU_DEFS.length);
  // Ids and full names (the editor title) are unchanged.
  assert.equal(MENU_DEFS.find((d) => d.id === 'feed.name')!.label, 'Names in feeds');
});

test('V111-10 a layout saved before the new defaults still resolves, with every row', async () => {
  const { group, item, SEP } = await import('../types.ts');
  const pick = (id: string) => MENUS.find((x) => x.def.id === id)!;
  // Names: the old Look up and Hide or block groups stay as the user saved them.
  const name = pick('chat.name');
  const oldName = {
    nodes: [
      item('usercard'),
      item('copy'),
      SEP,
      group('lookup', 'Look up', [item('search'), item('logs')], 'search'),
      group('hideblock', 'Hide or block', [item('hide'), item('block')], 'eyeOff'),
      SEP,
      group('moderate', 'Moderator', [item('timeout'), item('warn'), item('ban')], 'shield'),
    ],
    known: name.def.items.map((i) => i.id).filter((i) => i !== 'openChat'),
  };
  const got = resolveMenu(name.def, oldName, name.sample('mod'));
  const all = flat(got);
  for (const l of ['Open usercard', 'Copy name', 'Look up', 'Search their messages', 'Open their logs', 'Hide or block', 'Hide user for me', 'Block on Twitch', 'Time out user', 'Warn user', 'Ban user'])
    assert.ok(all.includes(l), l);
  assert.deepEqual(menuProblems(got), []);
  // Messages: a saved Pin message (broadcaster's line) row is gone, the one Pin row keeps working.
  const msg = pick('chat.message');
  const oldMsg = {
    nodes: [item('copy'), group('moderate', 'Moderator', [item('pin'), item('pinLine'), item('warn'), item('ban')], 'shield')],
    known: msg.def.items.map((i) => i.id),
  };
  assert.ok(flat(resolveMenu(msg.def, oldMsg, msg.sample('mod'))).includes('Pin message'));
  assert.ok(flat(resolveMenu(msg.def, oldMsg, { ...msg.sample('viewer'), pinLine: true })).includes('Pin message'));
  // Chat header: the old View group stays, Pop out chat joins after Overlay.
  const more = pick('chat.more');
  const oldMore = {
    nodes: [item('search'), item('overlay'), SEP, item('tile.theatre'), group('view', 'View', [item('tile.hideChat'), item('tile.panels')], 'layout'), item('tile.close')],
    known: more.def.items.map((i) => i.id).filter((i) => i !== 'popOut'),
  };
  assert.deepEqual(top(resolveMenu(more.def, oldMore, more.sample('viewer'))), ['Search', 'Overlay', 'Pop out chat', 'Theatre', 'View', 'Close']);
  // Emote: the saved 7TV group is kept.
  const emote = pick('chat.emote');
  const oldEmote = { nodes: [item('card'), group('seventv', '7TV', [item('edits')], 'seventv'), item('hide')], known: emote.def.items.map((i) => i.id) };
  assert.deepEqual(top(resolveMenu(emote.def, oldEmote, emote.sample('viewer'))), ['Open emote card', 'Add to 7TV channel', 'Hide emote']);
});

// V112-27 The chat modes menu.
const modesState = { slow: 0, followersOnly: null as number | null, subsOnly: false, emoteOnly: false, unique: false };
const modesMenu = (patch: Partial<typeof modesState> = {}, set: (c: unknown) => void = () => {}, ask: (k: string) => void = () => {}) => {
  const m = MENUS.find((x) => x.def.id === 'chat.modes')!;
  return resolveMenu(m.def, undefined, { state: { ...modesState, ...patch }, set, ask }) as { label: string; checked?: boolean; hint?: string; items?: unknown; field?: unknown; run?: () => void }[];
};

test('chat.modes: five rows, each unchecked with nothing on', () => {
  const list = modesMenu().filter((e) => (e as unknown) !== 'separator');
  assert.deepEqual(list.map((e) => e.label), ['Emote only', 'Subscribers only', 'Unique chat', 'Slow mode', 'Followers only']);
  assert.deepEqual(list.map((e) => !!e.checked), [false, false, false, false, false]);
});

test('chat.modes: checks follow ROOMSTATE', () => {
  const list = modesMenu({ emoteOnly: true, subsOnly: true, unique: true, slow: 30, followersOnly: 10 }).filter((e) => (e as unknown) !== 'separator');
  assert.deepEqual(list.map((e) => !!e.checked), [true, true, true, true, true]);
  assert.equal(list[3].hint, '30s');
  assert.equal(list[4].hint, '10m');
  // Any follower is 0 minutes and still on.
  assert.equal(modesMenu({ followersOnly: 0 }).find((e) => e.label === 'Followers only')!.hint, 'Any');
});

test('chat.modes: slow and followers open the length dialog when off, turn off when on', () => {
  const got: unknown[] = [];
  const asked: unknown[] = [];
  const off = modesMenu({}, (c) => got.push(c), (k) => asked.push(k));
  const slow = off.find((e) => e.label === 'Slow mode')!;
  const fol = off.find((e) => e.label === 'Followers only')!;
  // Plain items: no submenu, no field.
  assert.equal(slow.items, undefined);
  assert.equal(fol.items, undefined);
  assert.equal(slow.field, undefined);
  slow.run!();
  fol.run!();
  assert.deepEqual(asked, ['slow', 'followers']);
  assert.deepEqual(got, []);
  const on = modesMenu({ slow: 5, followersOnly: 30 }, (c) => got.push(c));
  on.find((e) => e.label === 'Slow mode')!.run!();
  on.find((e) => e.label === 'Followers only')!.run!();
  assert.deepEqual(got, [{ kind: 'slow', secs: null }, { kind: 'followers', mins: null }]);
  assert.equal(on.find((e) => e.label === 'Slow mode')!.items, undefined);
});

test('chat.modes: toggling a row hands over the change', () => {
  const got: unknown[] = [];
  const m = modesMenu({ subsOnly: true }, (c) => got.push(c));
  m.find((e) => e.label === 'Subscribers only')!.run!();
  m.find((e) => e.label === 'Emote only')!.run!();
  assert.deepEqual(got, [{ kind: 'subs', on: false }, { kind: 'emote', on: true }]);
});
