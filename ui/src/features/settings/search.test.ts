import assert from 'node:assert/strict';
import { test } from 'node:test';

import { ENTRIES, PAGES, type Entry, type Page } from './registry/index.ts';
import { buildIndex, fold, pieces, search, whereOf } from './search.ts';

const pages: Page[] = [
  { id: 'general', label: 'General', icon: 'sliders', group: 'App' },
  { id: 'chat', label: 'Chat', icon: 'chat', group: 'Chat' },
];
const entry = (id: string, label: string, more: Partial<Entry> = {}): Entry => ({ id, page: 'general', label, ...more });
const ids = (docs: ReturnType<typeof buildIndex>, q: string) => search(docs, q).map((h) => h.entry.id);

test('V103-13 match-sorter order on names', () => {
  const docs = buildIndex(
    [
      entry('a', 'Hide them'),
      entry('b', 'Show timestamps'),
      entry('c', 'Timestamps'),
      entry('d', 'Timestamps and more'),
      entry('e', 'Pastimes'),
      entry('f', 'Time based stamps'),
    ],
    pages,
  );
  // Equal, starts with, a word starts with. Letters in order (f) only count when nothing else does (S2).
  assert.deepEqual(ids(docs, 'timestamps'), ['c', 'd', 'b']);
  assert.deepEqual(ids(docs, 'stimes'), ['e']);
  // Word starts beat contains; of equal matches the shorter name comes first.
  assert.deepEqual(ids(docs, 'st'), ['f', 'e', 'c', 'b', 'd']);
  // No word matches at all: acronym, then letters in order.
  assert.deepEqual(ids(docs, 'ht'), ['a', 'b']);
  assert.ok(ids(docs, 'tmstmp').includes('c'));
  assert.deepEqual(ids(docs, '  '), []);
  assert.deepEqual(ids(docs, 'zz'), []);
});

test('V103-13 a name beats keywords, hints and the page it is on', () => {
  const docs = buildIndex(
    [
      entry('page', 'Font', { page: 'chat' }),
      entry('hint', 'Size', { description: 'How big chat text is.' }),
      entry('word', 'Other', { keywords: ['chat text'] }),
      entry('name', 'Chat font'),
    ],
    pages,
  );
  assert.deepEqual(ids(docs, 'chat'), ['name', 'word', 'hint', 'page']);
});

test('V103-13 hints count from contains up', () => {
  const docs = buildIndex([entry('a', 'Delay', { description: 'Hold messages back a while.' })], pages);
  assert.deepEqual(ids(docs, 'back a'), ['a']);
  // Letters in order or an acronym in a hint are no match.
  assert.deepEqual(ids(docs, 'hmb'), []);
});

test('V103-13 several words: the phrase first, then every word anywhere', () => {
  const docs = buildIndex(
    [
      entry('split', 'Size', { page: 'chat', keywords: ['font'] }),
      entry('phrase', 'Chat font size'),
      entry('half', 'Font'),
    ],
    pages,
  );
  assert.deepEqual(ids(docs, 'font size'), ['phrase', 'split']);
  // Each by its weakest word: a page name counts as contains at most.
  assert.deepEqual(ids(docs, 'size  chat'), ['phrase', 'split']);
});

test('V103-13 ties keep the list order, pages first', () => {
  const docs = buildIndex([entry('b', 'Sound', { page: 'chat' }), entry('a', 'Sound'), entry('x', 'Sound', { page: 'gone' as Page['id'] })], pages);
  assert.deepEqual(ids(docs, 'sound'), ['a', 'b']);
});

test('V103-13 accents and case do not matter', () => {
  assert.equal(fold('  Théme   café '), 'Theme cafe');
  const docs = buildIndex([entry('a', 'Theme')], pages);
  assert.deepEqual(ids(docs, 'THÉME'), ['a']);
});

test('V103-13 the real registry finds the obvious', () => {
  const docs = buildIndex(ENTRIES, PAGES);
  const first = (q: string) => search(docs, q)[0]?.entry.id;
  assert.equal(first('timestamps'), 'chat.timestamps.show');
  assert.equal(first('dark mode'), 'appearance.theme');
  assert.equal(first('font size'), 'general.chatFontSize');
  assert.equal(first('block ads'), 'player.blockAds');
  assert.equal(first('streamer'), 'general.streamer.when');
  // Results say where they live.
  const hit = search(docs, 'paints')[0];
  assert.equal(`${hit.page.group} › ${hit.page.label}`, 'Chat › Emotes and badges');
});

test('S3 marks what matched, and the breadcrumb drops a repeat', () => {
  assert.deepEqual(pieces('Twitch badges', 'badge'), [
    { text: 'Twitch ', hit: false },
    { text: 'badge', hit: true },
    { text: 's', hit: false },
  ]);
  assert.deepEqual(pieces('Chat font size', 'size font'), [
    { text: 'Chat ', hit: false },
    { text: 'font', hit: true },
    { text: ' ', hit: false },
    { text: 'size', hit: true },
  ]);
  assert.deepEqual(pieces('Delay', 'zz'), [{ text: 'Delay', hit: false }]);
  assert.equal(whereOf(pages[1]), 'Chat');
  assert.equal(whereOf({ ...pages[0], group: 'App' }), 'App › General');
});
