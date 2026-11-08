import assert from 'node:assert/strict';
import { test } from 'node:test';

import { GROUPS, groupOf, masterOn, masterStates, patchFor, read, summary } from './emoteGroups.ts';

const seventv = GROUPS.find((g) => g.key === 'seventv')!;
const v = (on: { e: boolean; b: boolean; p: boolean }) => ({
  chat: { seventvEmotes: on.e, badges: { seventv: on.b }, paints: { seventv: on.p } },
  emotes: {},
});

test('V110-E13 a child reads from its stored key', () => {
  assert.equal(read(v({ e: true, b: false, p: true }), seventv.children[1]), false);
});

test('V110-E13 the group switch is on while any child is', () => {
  assert.equal(masterOn(v({ e: false, b: false, p: true }), seventv), true);
  assert.equal(masterOn(v({ e: false, b: false, p: false }), seventv), false);
});

test('V110-E13 off turns all off; on restores what was on, or everything', () => {
  assert.ok(masterStates(seventv, false).every(([, on]) => !on));
  const back = masterStates(seventv, true, { 'chat.seventvEmotes': true, 'chat.paints.seventv': true });
  assert.deepEqual(
    back.map(([, on]) => on),
    [true, false, true],
  );
  assert.ok(masterStates(seventv, true, {}).every(([, on]) => on));
  assert.ok(masterStates(seventv, true).every(([, on]) => on));
});

test('V110-E13 patches nest per partition', () => {
  const other = GROUPS.find((g) => g.key === 'other')!;
  assert.deepEqual(patchFor(masterStates(seventv, false)), {
    chat: { seventvEmotes: false, badges: { seventv: false }, paints: { seventv: false } },
  });
  assert.deepEqual(patchFor([[other.children[0], true]]), { emotes: { badgeSets: { ffzap: true } } });
});

test('V110-E13 a closed group sums up its switches', () => {
  assert.equal(summary(v({ e: true, b: true, p: false }), seventv), 'Emotes, badges · name paints off');
  assert.equal(summary(v({ e: true, b: true, p: true }), seventv), 'Emotes, badges, name paints');
  assert.equal(summary(v({ e: false, b: false, p: false }), seventv), 'All off');
});

test('V110-E13 the search finds the group of a setting', () => {
  assert.equal(groupOf('chat.badges.ffz')?.key, 'ffz');
  assert.equal(groupOf('chat.paints.outlineLight')?.key, 'seventv');
  assert.equal(groupOf('emotes.badgeSets.homies')?.key, 'other');
  assert.equal(groupOf('chat.emoteSize'), undefined);
});
