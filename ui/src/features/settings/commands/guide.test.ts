// V106-13 guide.ts is generated from docs/commands/registry.json.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { GUIDE } from './guide.ts';

test('guide.ts matches registry.json', () => {
  const file = JSON.parse(readFileSync(new URL('../../../../../docs/commands/registry.json', import.meta.url), 'utf8'));
  const want = Object.fromEntries(file.guide.map((g: { id: string }) => [g.id, g]));
  assert.deepEqual(GUIDE, want, 'guide.ts is stale: run tools/commands/registry.ps1');
});
