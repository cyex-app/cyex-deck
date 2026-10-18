import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { EditForm } from './run.ts';
import { afterEdit } from './renamed.ts';

const emote = { provider: 'seventv', id: '01T', name: 'TeaTime', animated: false };
const rename: EditForm = { kind: 'rename', here: { setId: 's', channel: 'mockme', alias: 'TeaTime' } as never };
const add: EditForm = { kind: 'add', target: { setId: 's2', channel: 'other' } as never };

test('a rename makes the card show and look up the new name', () => {
  const next = afterEdit(emote, rename, 'TeaTimeZ');
  assert.equal(next.name, 'TeaTimeZ');
  assert.equal(next.id, '01T');
  assert.equal(emote.name, 'TeaTime');
});

test('renaming back works the same way', () => {
  assert.equal(afterEdit({ ...emote, name: 'TeaTimeZ' }, rename, 'TeaTime').name, 'TeaTime');
});

test('an add or the same name keeps the shown emote', () => {
  assert.equal(afterEdit(emote, add, 'TeaTimeZ'), emote);
  assert.equal(afterEdit(emote, rename, 'TeaTime'), emote);
});
