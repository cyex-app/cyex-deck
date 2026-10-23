import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Tab } from '../../ipc/bindings/Tab';
import { bumped, cleared, mentionNews } from './feedNews.ts';

const feedTab = (feed: string) => ({ columns: [{ tiles: [{ feed }] }] }) as unknown as Tab;
const chatTab = { columns: [{ tiles: [{ feed: null }] }] } as unknown as Tab;

test('lines in a feed nobody shows are counted (V107-H15)', () => {
  const mentions = feedTab('Mentions');
  let c = bumped({}, 'mentions', [chatTab]);
  c = bumped(c, 'mentions', [chatTab]);
  assert.equal(mentionNews(c, mentions), 2);
  assert.equal(mentionNews(c, chatTab), 0);
});

test('a shown feed is not counted, showing it clears the count', () => {
  const mentions = feedTab('Mentions');
  assert.deepEqual(bumped({}, 'mentions', [mentions]), {});
  const c = { mentions: 3, pings: 1 };
  assert.deepEqual(cleared(c, mentions), { pings: 1 });
  assert.equal(cleared(c, chatTab), c);
});

test('only the Mentions feed lights its tab', () => {
  assert.equal(mentionNews({ pings: 4 }, feedTab('Pings')), 0);
});
