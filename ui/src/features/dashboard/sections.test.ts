import assert from 'node:assert/strict';
import { test } from 'node:test';

import { FIRST, keepSection, lastSection, SECTIONS, sectionOr } from './sections.ts';

test('V109-C04 a kept section opens again; an unknown one opens the first', () => {
  assert.equal(sectionOr('rewards'), 'rewards');
  assert.equal(sectionOr('nope'), FIRST);
  assert.equal(sectionOr(null), FIRST);
  assert.equal(new Set(SECTIONS.map((s) => s.id)).size, SECTIONS.length);
});

test('V109-C04 the last section survives storage, and no storage is fine', () => {
  const g = globalThis as { localStorage?: unknown };
  const before = g.localStorage;
  try {
    delete g.localStorage;
    assert.equal(lastSection(), FIRST);
    keepSection('videos');
    const kept = new Map<string, string>();
    g.localStorage = { getItem: (k: string) => kept.get(k) ?? null, setItem: (k: string, v: string) => void kept.set(k, v) };
    keepSection('modset');
    assert.equal(lastSection(), 'modset');
  } finally {
    if (before === undefined) delete g.localStorage;
    else g.localStorage = before;
  }
});
