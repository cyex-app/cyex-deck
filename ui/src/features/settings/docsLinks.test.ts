import assert from 'node:assert/strict';
import { test } from 'node:test';

import { DOCS, docsUrl, type DocsAnchor } from './docsLinks.ts';
import { PAGES } from './registry/pages.ts';

const keys = Object.keys(DOCS) as DocsAnchor[];

test('every docs link points at the docs page (V106-12)', () => {
  for (const k of keys) assert.ok(docsUrl(k).startsWith('https://cyex.app/deck/docs/#'));
});

test('every s-<id> anchor names a real settings page', () => {
  const ids = new Set(PAGES.map((p) => p.id));
  for (const k of keys) {
    const a: string = DOCS[k];
    if (a.startsWith('s-')) assert.ok(ids.has(a.slice(2)), a);
  }
});
