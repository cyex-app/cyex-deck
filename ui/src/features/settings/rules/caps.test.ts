import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { CAPS } from './caps.ts';

const rust = (path: string) => readFileSync(new URL(`../../../../../crates/settings/src/schema/${path}`, import.meta.url), 'utf8');
const num = (src: string, name: string) => {
  const m = src.match(new RegExp(`const ${name}: usize = ([0-9_]+);`));
  assert.ok(m, `${name} found in Rust`);
  return Number(m[1].replaceAll('_', ''));
};

test('V110-F101 list caps match the Rust constants', () => {
  const commands = rust('commands.rs');
  const highlights = rust('highlights/mod.rs');
  assert.equal(CAPS.commandLines, num(commands, 'MAX_LINES'));
  assert.equal(CAPS.automations, num(commands, 'MAX_AUTOMATIONS'));
  assert.equal(CAPS.filters, num(highlights, 'MAX_FILTERS'));
  assert.equal(CAPS.phrases, num(highlights, 'MAX_WORDS'));
  assert.equal(CAPS.globalList, num(highlights, 'MAX_GLOBAL'));
  assert.equal(CAPS.names, num(highlights, 'MAX_NEVER'));
  assert.equal(CAPS.rules, num(highlights, 'MAX_RULES'));
});
