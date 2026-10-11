import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { test } from 'node:test';

import { ENTRIES, hotkeyEntries, PAGES } from './index.ts';
import { UNLISTED } from './unlisted.ts';

const BINDINGS = new URL('../../../ipc/bindings/', import.meta.url);
const SETTINGS_IPC = new URL('../../../ipc/settings.ts', import.meta.url);
const SETTINGS_UI = new URL('../', import.meta.url);

/** The object body of a generated type, or null when it isn't an object (an enum, a list). */
function objectType(name: string): string | null {
  let src: string;
  try {
    src = readFileSync(new URL(`${name}.ts`, BINDINGS), 'utf8');
  } catch {
    return null;
  }
  src = src.replace(/\/\*[\s\S]*?\*\//g, '').replace(/^\s*(\/\/|import ).*$/gm, '');
  const body = src.match(/export type \w+ = ([\s\S]*);\s*$/)?.[1].trim();
  return body?.startsWith('{') && !body.startsWith('{ [key') ? body : null;
}

/** `name: Type` pairs at the top level of an object type. */
function fields(body: string): { name: string; type: string }[] {
  const out: string[] = [];
  let depth = 0;
  let cur = '';
  for (const ch of body.slice(1, body.lastIndexOf('}'))) {
    if ('{<(['.includes(ch)) depth++;
    if ('}>)]'.includes(ch)) depth--;
    if (ch === ',' && depth === 0) {
      out.push(cur);
      cur = '';
    } else cur += ch;
  }
  out.push(cur);
  return out
    .map((f) => f.trim())
    .filter(Boolean)
    .map((f) => {
      const at = f.indexOf(':');
      return { name: f.slice(0, at).replace('?', '').trim(), type: f.slice(at + 1).trim() };
    });
}

/** Every stored setting as `partition.field.field`, read from the ts-rs bindings, so a
 * field added in Rust shows up here once the bindings are regenerated. */
function settingKeys(): string[] {
  const ipc = readFileSync(SETTINGS_IPC, 'utf8');
  const values = ipc.match(/export interface SettingsValues \{([\s\S]*?)\n\}/)?.[1] ?? '';
  const partitions = [...values.matchAll(/^\s*(\w+): (\w+);/gm)].map((m) => [m[1], m[2]] as const);
  assert.ok(partitions.length >= 17, 'read the partitions from SettingsValues');
  const keys: string[] = [];
  const walk = (prefix: string, body: string) => {
    for (const f of fields(body)) {
      const key = `${prefix}.${f.name}`;
      const inner = /^[A-Z]\w*$/.test(f.type) ? objectType(f.type) : null;
      if (inner) walk(key, inner);
      else keys.push(key);
    }
  };
  for (const [partition, type] of partitions) {
    const body = objectType(type);
    assert.ok(body, `read bindings/${type}.ts`);
    walk(partition, body);
  }
  return keys;
}

const KEYS = settingKeys();
const under = (key: string, prefix: string) => key === prefix || key.startsWith(`${prefix}.`);
const isStored = (id: string) => !id.includes(':');

test('V103-13 every setting has a registry entry', () => {
  const covered = new Set(ENTRIES.flatMap((e) => [e.id, ...(e.keys ?? [])]));
  const unlisted = Object.keys(UNLISTED);
  const missing = KEYS.filter((k) => !covered.has(k) && !unlisted.some((u) => under(k, u)));
  assert.deepEqual(
    missing,
    [],
    'Add these to the registry (settings/registry/<group>.ts) with setting="<id>" on their row, or to unlisted.ts if Settings never shows them',
  );
});

test('V103-13 registry entries point at settings that exist', () => {
  const exists = (id: string) => KEYS.some((k) => under(k, id));
  const stale = [
    ...ENTRIES.filter((e) => isStored(e.id)).map((e) => e.id),
    ...ENTRIES.flatMap((e) => e.keys ?? []),
    ...Object.keys(UNLISTED),
  ].filter((id) => !exists(id));
  assert.deepEqual(stale, []);
  // An entry's own key needs no `keys`, and nothing is both listed and unlisted.
  for (const e of ENTRIES) {
    assert.ok(!(e.keys ?? []).includes(e.id), e.id);
    assert.ok(!Object.keys(UNLISTED).some((u) => under(e.id, u)), `${e.id} is unlisted`);
  }
});

test('V103-13 entries are unique, on real pages, with a name', () => {
  const all = [...ENTRIES, ...hotkeyEntries({ 'tabs.new': { label: 'Add a tab', default: 'Ctrl+T' } })];
  const ids = all.map((e) => e.id);
  assert.equal(new Set(ids).size, ids.length, 'unique ids');
  const pages = new Set(PAGES.map((p) => p.id));
  for (const e of all) {
    assert.ok(pages.has(e.page), `${e.id}: page ${e.page}`);
    assert.ok(e.label.trim(), `${e.id}: label`);
    if (e.anchor) assert.ok(ids.includes(e.anchor), `${e.id}: anchor ${e.anchor}`);
  }
  // Every page can be found by something on it, so search reaches all of them.
  for (const p of PAGES) assert.ok(all.some((e) => e.page === p.id), `${p.id} has entries`);
});

test('S2 hotkeys are entries of their own', () => {
  const [local, everywhere] = hotkeyEntries({
    'tabs.new': { label: 'Add a tab', default: 'Ctrl+T' },
    'global.show': { label: 'Show or hide cyex deck', default: '', global: true },
  });
  assert.equal(local.id, 'hotkey:tabs.new');
  assert.equal(local.page, 'hotkeys');
  assert.equal(local.description, 'Ctrl+T unless you change it.');
  assert.equal(everywhere.description, 'Works while another app is in front.');
});

/** `setting=` and `data-setting=` values in the settings pages, as patterns: `{x}` and
 * `${x}` parts match any word. */
function markupAnchors(): RegExp[] {
  const out: RegExp[] = [];
  const files = readdirSync(SETTINGS_UI, { recursive: true, encoding: 'utf8' }).filter((f) => f.endsWith('.svelte'));
  assert.ok(files.length > 20, 'read the settings pages');
  for (const file of files) {
    const src = readFileSync(new URL(file.replaceAll('\\', '/'), SETTINGS_UI), 'utf8');
    for (const m of src.matchAll(/\b(?:data-)?setting=(?:"([^"]*)"|\{`([^`]*)`\}|\{'([^']*)'\})/g)) {
      for (const word of (m[1] ?? m[2] ?? m[3]).split(/\s+/).filter(Boolean)) {
        const parts = word.split(/\$?\{[^}]*\}/).map((p) => p.replace(/[.*+?^$()|[\]\\]/g, '\\$&'));
        out.push(new RegExp(`^${parts.join('\\S+')}$`));
      }
    }
  }
  return out;
}

test('V103-13 every entry has a place to jump to', () => {
  const anchors = markupAnchors();
  const marked = (id: string) => anchors.some((a) => a.test(id));
  const lost = ENTRIES.filter((e) => !marked(e.id) && !(e.anchor && marked(e.anchor))).map((e) => e.id);
  assert.deepEqual(lost, [], 'Put setting="<id>" on the row (or data-setting on the element)');
  assert.ok(marked('hotkey:tabs.new'), 'hotkey rows');
});
