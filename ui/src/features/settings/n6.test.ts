// V110-N6 Alerts rename, own menu entries on the Menus page, a hint under every page title.
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { test } from 'node:test';

import { ENTRIES } from './registry/index.ts';
import { PAGES } from './registry/pages.ts';

const read = (rel: string) => readFileSync(new URL(rel, import.meta.url), 'utf8');

test('the Notifications page is called Alerts, id unchanged', () => {
  const p = PAGES.find((x) => x.id === 'notifications');
  assert.equal(p?.label, 'Alerts');
  assert.match(read('./sections/NotificationsSection.svelte'), /title="Alerts" level=\{2\}/);
});

test('the bell button is Twitch notifications', () => {
  assert.doesNotMatch(read('../bell/BellButton.svelte'), /label="Notifications"/);
  assert.match(read('../bell/BellButton.svelte'), /label="Twitch notifications"/);
});

test('menu entries and usercard buttons search to Menus; Commands keeps commands and timers', () => {
  for (const id of ['commands.userMenu', 'commands.messageMenu', 'commands.channelMenu', 'commands.usercardButtons']) {
    assert.equal(ENTRIES.find((e) => e.id === id)?.page, 'menus', id);
  }
  assert.equal(ENTRIES.find((e) => e.id === 'commands.lines')?.page, 'commands');
  const commands = read('./sections/CommandsSection.svelte');
  assert.doesNotMatch(commands, /usercardButtons|userMenu/);
  assert.match(commands, /Menu entries and usercard buttons are on the/);
  assert.match(read('./sections/MenusSection.svelte'), /<OwnEntries \/>/);
});

test('every page heading has a hint', () => {
  const dir = new URL('./sections/', import.meta.url);
  for (const f of readdirSync(dir).filter((n) => n.endsWith('Section.svelte'))) {
    const src = readFileSync(new URL(f, dir), 'utf8');
    for (const m of src.matchAll(/<SectionHead title="[^"]+" level=\{2\}[^>]*?(\/)?>/g)) {
      assert.ok(!m[1], `${f}: heading without a hint`);
    }
  }
});
