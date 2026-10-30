import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ChannelCard } from '../../ipc/bindings/ChannelCard';
import { cardHidden, categoryHidden, visibleCards, visibleCategories, withHidden, withoutHidden, withTag } from './hidden.ts';

const hidden = { categories: [{ id: '1', name: 'Slots' }], tags: ['NSFW'] };
const none = { categories: [], tags: [] };

function card(id: string, game: { id: string; name: string } | null, tags: string[] = []): ChannelCard {
  return {
    id,
    login: id,
    name: id,
    avatar: null,
    stream: {
      title: '',
      viewers: 1,
      startedAt: '2026-10-30T10:00:00Z',
      rerun: false,
      game: game && { ...game, slug: game.name.toLowerCase() },
      tags,
    },
    lastLive: null,
    lastGame: null,
    lastTitle: null,
  };
}

test('V108-N05 a category matches by id, or by name when there is no id', () => {
  assert.ok(categoryHidden(hidden, { id: '1', name: 'Renamed' }));
  assert.ok(categoryHidden(hidden, { name: ' slots ' }));
  assert.ok(!categoryHidden(hidden, { id: '2', name: 'Chess' }));
  assert.ok(!categoryHidden(none, { id: '1', name: 'Slots' }));
  assert.ok(!categoryHidden(hidden, { id: null, name: null }));
});

test('V108-N05 live channels go by category and tag; offline ones stay', () => {
  const list = [
    card('a', { id: '1', name: 'Slots' }),
    card('b', { id: '2', name: 'Chess' }, ['nsfw']),
    card('c', { id: '2', name: 'Chess' }, ['Speedrun']),
    card('d', null),
    { ...card('e', null), stream: null },
  ];
  assert.deepEqual(visibleCards(hidden, list).map((c) => c.id), ['c', 'd', 'e']);
  // Inside the hidden category itself only tags apply.
  assert.ok(!cardHidden(hidden, list[0], { game: false }));
  assert.equal(visibleCards(none, list), list);
});

test('V108-N05 categories are filtered, hiding and unhiding keep the list tidy', () => {
  const cats = [{ id: '1', name: 'Slots' }, { id: '2', name: 'Chess' }];
  assert.deepEqual(visibleCategories(hidden, cats), [cats[1]]);
  let list = withHidden([], { id: '1', name: 'Slots' });
  list = withHidden(list, { id: '1', name: 'Slots again' });
  list = withHidden(list, { id: '', name: 'No id' });
  assert.deepEqual(list, [{ id: '1', name: 'Slots' }]);
  assert.deepEqual(withoutHidden(list, '1'), []);
});

test('V108-N05 tags are added once, without a leading #', () => {
  assert.deepEqual(withTag(['NSFW'], ' #nsfw '), ['NSFW']);
  assert.deepEqual(withTag(['NSFW'], '#18+'), ['NSFW', '18+']);
  assert.deepEqual(withTag([], '  '), []);
});
