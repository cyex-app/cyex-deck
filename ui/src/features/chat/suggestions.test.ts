import assert from 'node:assert/strict';
import { test } from 'node:test';

import { commandItems, emoteItems, viewerLevel } from './suggestions.ts';

test('emotes before emoji', () => {
  const items = emoteItems(
    'ka',
    [{ name: 'Kappa', provider: 'twitch', id: '25' }],
    [{ char: '🍢', name: 'oden' }],
  );
  assert.deepEqual(
    items.map((i) => i.label),
    ['Kappa', ':oden:'],
  );
  assert.equal(items[1].value, '🍢');
});

test('an exact emote name comes first, then the rest A-Z (V110-U71)', () => {
  const emotes = ['Kappu', 'KappaWealth', 'Kappa', 'KappaPride'].map((name, i) => ({ name, provider: 'twitch' as const, id: String(i) }));
  assert.deepEqual(emoteItems('Kappa', emotes, []).map((i) => i.label), ['Kappa', 'KappaPride', 'KappaWealth']);
  assert.equal(emoteItems('kappa', emotes, [])[0].label, 'Kappa');
  assert.deepEqual(emoteItems('kapp', emotes, []).map((i) => i.label), ['Kappa', 'KappaPride', 'KappaWealth', 'Kappu']);
});

test('emotes come before emoji; an emoji leads only on an exact name (V110-U71)', () => {
  const emotes = ['melon', 'ME?', 'menheraLost'].map((name, i) => ({ name, provider: 'seventv' as const, id: String(i) }));
  const emoji = [{ name: 'mechanic', char: '🧑‍🔧' }, { name: 'melting_face', char: '🫠' }];
  assert.deepEqual(emoteItems('me', emotes, emoji).map((i) => i.label), ['ME?', 'melon', 'menheraLost', ':mechanic:', ':melting_face:']);
  const sob = emoteItems('sob', [{ name: 'SoBayed', provider: 'bttv' as const, id: 's' }], [{ name: 'sob', char: '😭' }]);
  assert.deepEqual(sob.map((i) => i.label), [':sob:', 'SoBayed']);
});

test('one name from two providers: the preferred provider is offered (V110-U71)', () => {
  const emotes = [
    { name: 'OMEGALUL', provider: 'twitch' as const, id: '1' },
    { name: 'OMEGALUL', provider: 'seventv' as const, id: '2' },
    { name: 'OMEGALUL', provider: 'bttv' as const, id: '3' },
  ];
  const order = (p: string) => ['seventv', 'twitch', 'bttv', 'ffz'].indexOf(p);
  const items = emoteItems('omega', emotes, [], 8, order);
  assert.equal(items.length, 1);
  assert.equal(items[0].emote?.provider, 'seventv');
  assert.equal(emoteItems('omega', emotes, [], 8, (p) => ['twitch', 'seventv', 'bttv', 'ffz'].indexOf(p))[0].emote?.provider, 'twitch');
});

test('bot commands by level', () => {
  const cmds = [
    { name: '!discord', bot: 'nightbot', level: 0, args: [] },
    { name: '!dead', bot: 'fossabot', level: 4, args: [] },
    { name: '!so', bot: 'streamelements', level: 4, args: ['User'] },
  ];
  assert.deepEqual(
    commandItems('!d', cmds, 0).map((i) => i.value),
    ['!discord'],
  );
  assert.equal(commandItems('!s', cmds, viewerLevel({ role: 'moderator', vip: false, sub: false }))[0].label, '!so <User>');
  // V102-21 Lead mods count as mods, the channel's own account as the streamer.
  assert.equal(viewerLevel({ role: 'leadModerator', vip: false, sub: false }), 4);
  assert.equal(viewerLevel({ role: 'broadcaster', vip: false, sub: false }), 5);
});

test('an emoji named exactly what is typed comes before emotes that only start with it (V110-F10)', () => {
  const emotes = ['SoBayed', 'CoolStoryBob', 'Isaboobs3'].map((name, i) => ({ name, provider: 'twitch' as const, id: String(i) }));
  const items = emoteItems('sob', emotes, [{ char: '😭', name: 'sob' }]);
  assert.equal(items[0].glyph, '😭');
  assert.equal(items[1].label, 'SoBayed');
});

test('letters merely in order are no match for `:`, an acronym is (V110-F49)', () => {
  const emotes = ['CoolStoryBob', 'cdprSonbird', 'SoBayed', 'SmallOrangeBird'].map((name, i) => ({ name, provider: 'twitch' as const, id: String(i) }));
  assert.deepEqual(
    emoteItems('sob', emotes, []).map((i) => i.label),
    ['SoBayed', 'SmallOrangeBird'],
  );
});
