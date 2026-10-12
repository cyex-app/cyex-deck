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
