// V1051-04 docs/commands/registry.json: the sections, features, changelog, settings and hotkeys
// against this registry, hotkeys.svelte.ts, docs/commands/features.json and changelog.json.
// Commands, variables and functions are checked in Rust (app/src/chat/run/registry/tests.rs).

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

// @ts-expect-error plain JS module outside ui/
import { build, REGISTRY } from '../../../../../tools/commands/sections.mjs';

type Feature = {
  id: string;
  section: string;
  title: string;
  summary: string;
  where: string;
  settings: string[];
  hotkeys: string[];
  commands: string[];
  since: string;
  refs: string[];
};
type Version = { version: string; date: string; released: boolean; highlights: { text: string; features: string[] }[] };

const STALE = 'docs/commands/registry.json is stale: run tools/commands/registry.ps1';
const file = JSON.parse(readFileSync(REGISTRY, 'utf8'));
const now = await build();
const features = now.features as Feature[];
const changelog = now.changelog as Version[];
const versions = changelog.map((v) => v.version);

test('V1051-04 registry.json settings match the settings registry', () => {
  assert.deepEqual(file.settings, now.settings, STALE);
  for (const s of now.settings as { id: string; since: string; status: string }[]) {
    assert.ok(versions.includes(s.since), `${s.id}: since ${s.since} is a changelog version`);
    assert.ok(['', 'new', 'changed'].includes(s.status), `${s.id}: status`);
  }
});

test('V1051-04 registry.json hotkeys match the hotkey list', () => {
  assert.deepEqual(file.hotkeys, now.hotkeys, STALE);
  const ids = now.hotkeys.map((h: { id: string }) => h.id);
  assert.equal(new Set(ids).size, ids.length, 'unique hotkey ids');
  for (const h of now.hotkeys as { id: string; since: string }[]) {
    assert.ok(versions.includes(h.since), `${h.id}: since ${h.since} is a changelog version`);
  }
});

test('V1051-04 registry.json features and changelog match features.json and changelog.json', () => {
  assert.deepEqual(file.sections, now.sections, STALE);
  assert.deepEqual(file.features, now.features, STALE);
  assert.deepEqual(file.changelog, now.changelog, STALE);
  assert.equal(file.current, now.current, STALE);
  assert.equal(file.statusSince, now.statusSince, STALE);
});

test('V1051-04 every feature reference exists', () => {
  const settingIds = new Set((now.settings as { id: string; keys: string[] }[]).flatMap((s) => [s.id, ...s.keys]));
  const hotkeyIds = new Set((now.hotkeys as { id: string }[]).map((h) => h.id));
  const commandNames = new Set((file.commands as { name: string; aliases: string[] }[]).flatMap((c) => [c.name, ...c.aliases]));
  const sectionIds = new Set((now.sections as { id: string }[]).map((s) => s.id));
  const ids = features.flatMap((f) => [f.id, ...f.refs]);
  assert.equal(new Set(ids).size, ids.length, 'feature ids and refs are unique');
  for (const f of features) {
    assert.ok(sectionIds.has(f.section), `${f.id}: section ${f.section}`);
    assert.ok(versions.includes(f.since), `${f.id}: since ${f.since} is a changelog version`);
    for (const s of f.settings) assert.ok(settingIds.has(s), `${f.id}: setting ${s}`);
    for (const h of f.hotkeys) assert.ok(hotkeyIds.has(h), `${f.id}: hotkey ${h}`);
    for (const c of f.commands) assert.ok(commandNames.has(c), `${f.id}: command /${c}`);
  }
});

test('V1051-04 every changelog link points to a feature', () => {
  const featureIds = new Set(features.map((f) => f.id));
  assert.equal(new Set(versions).size, versions.length, 'unique versions');
  assert.equal(now.current, versions.at(-1), 'current is the newest version');
  for (const v of changelog) {
    assert.ok(v.highlights.length >= 1 && v.highlights.length <= 15, `${v.version}: 1 to 15 highlights`);
    for (const h of v.highlights) for (const id of h.features) assert.ok(featureIds.has(id), `${v.version}: feature ${id}`);
  }
});

test('V1051-04 features and changelog read as site text', () => {
  const texts: [string, string][] = [
    ...features.flatMap((f): [string, string][] => [f.title, f.summary, f.where].map((t) => [f.id, t])),
    ...changelog.flatMap((v) => v.highlights.map((h): [string, string] => [v.version, h.text])),
    ...(now.sections as { id: string; label: string }[]).map((s): [string, string] => [s.id, s.label]),
  ];
  for (const [id, text] of texts) {
    assert.ok(!text.includes("'"), `${id}: curly ’, not '`);
    assert.ok(!/\d{4}-\d{2}-\d{2}/.test(text), `${id}: dates as yyyy/mm/dd`);
  }
  for (const v of changelog) assert.match(v.date, /^\d{4}\/\d{2}\/\d{2}$/, `${v.version}: date as yyyy/mm/dd`);
  for (const f of features) {
    const sentences = f.summary.split(/(?<=[.!?”])\s+(?=[A-Z“])/).length;
    assert.ok(sentences <= 3, `${f.id}: 1 to 3 sentences, has ${sentences}`);
  }
});
