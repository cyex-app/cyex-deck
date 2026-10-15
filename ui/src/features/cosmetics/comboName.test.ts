import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { cleanName } from './comboName.ts';

const FIXTURE = new URL('../../../../crates/settings/tests/fixtures/combo_names.json', import.meta.url);
const cases = JSON.parse(readFileSync(FIXTURE, 'utf8')) as { in: string; out?: string; err?: string }[];

test('the shared fixture', () => {
  for (const c of cases) {
    const r = cleanName(c.in);
    if (c.err) assert.ok(!r.ok && r.error === c.err, `${JSON.stringify(c.in)} should be ${c.err}`);
    else assert.ok(r.ok && r.name === c.out, `${JSON.stringify(c.in)} should be ${c.out}`);
  }
});

test('messages are Rust’s', () => {
  const r = cleanName('Save');
  assert.ok(!r.ok);
  assert.equal(r.message, '“Save” is part of /combo. Pick another name.');
  const e = cleanName(' ');
  assert.ok(!e.ok);
  assert.equal(e.message, 'Give it a name.');
});
