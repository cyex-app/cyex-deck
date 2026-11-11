// Overnight 2026/11/11 F2: source scans for the fixes that live in Svelte files or Tauri-bound modules.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const read = (p: string) => readFileSync(new URL(p, import.meta.url), 'utf8');
const data = read('./sections/DataSection.svelte');

test('A2-1 apply, restore and broken restore share one busy guard', () => {
  for (const fn of ['restoreBroken', 'applyImport', 'restore']) {
    const body = data.slice(data.indexOf(`async function ${fn}(`)).split('\n  }\n')[0];
    assert.match(body, /if \(busy\) return;/, fn);
    assert.match(body, /finally \{\s*busy = false;/, fn);
  }
  assert.ok(!/let keptOn/.test(data), 'the kept flag travels with the kept code');
});

test('A2-2 a pending import review is cancelled when the section goes away', () => {
  assert.match(data, /\$effect\(\(\) => \(\) => \{\s*if \(review\) void settingsIpc\.cancelImport/);
});

test('A2-3 Undo does not leave a rejected patch unhandled', () => {
  assert.match(data, /attempt\(\(\) => settings\.patch\('commands', \{ automationsOn: true \}\)/);
});

test('N3-2 a confirm bubble acts once', () => {
  const s = read('../../components/ConfirmButton.svelte');
  assert.match(s, /onconfirm=\{\(\) => \{[^}]*if \(!armed\) return;/s);
});

test('N3-6 the menu editor hint is three short lines, plus the placeholders when the menu has entries', () => {
  const s = read('./menus/MenuEditor.svelte');
  const help = s.slice(s.indexOf('<ul class="help">'), s.indexOf('</ul>', s.indexOf('<ul class="help">')));
  assert.equal((help.match(/<li>/g) ?? []).length, 4);
  assert.ok(help.includes('{#if chips.length}<li>Your own entries can use'));
  assert.ok(help.includes('F2 renames, Space hides, Delete removes, Alt with the arrows moves.'));
});

test('N3-7 the wallpaper question reads short', () => {
  const s = read('./ThemeShare.svelte');
  assert.ok(s.includes('Your wallpaper is on this PC. To put it in the code, it’s uploaded to cyex.app, and anyone with the code can open it.'));
  assert.match(s, /A2-9[\s\S]*void theme\.id;\s*asking = false;/);
});

test('A2-4 the chat button comes back when theatre is entered again', () => {
  const s = read('../channel/StreamSide.svelte');
  assert.match(s, /\$effect\(\(\) => \{\s*if \(!chatButton\) still\(\);/);
});

test('A2-5 A2-6 the taskbar room goes off through the fullscreen queue, before leaving fullscreen', () => {
  const w = read('../tabs/windowFullscreen.ts');
  assert.ok(w.indexOf('taskbarRoom(false)') < w.indexOf('setFullscreen(false)'));
  assert.match(w, /if \(!want\) before = null;/);
  const t = read('../tabs/theatreTaskbar.svelte.ts');
  assert.match(t, /windowFullscreen\.taskbarRoomOff\(\)/);
  assert.ok(!/stop[\s\S]*void windowsIpc\.taskbarRoom\(false\)/.test(t.slice(t.indexOf('return () =>'))));
});

test('A2-10 a menu drag ends on blur and on unmount', () => {
  const s = read('./menus/MenuMock.svelte');
  assert.match(s, /addEventListener\('blur', blur\)/);
  assert.match(s, /removeEventListener\('blur', blur\)/);
  assert.match(s, /\$effect\(\(\) => \(\) => stopDrag\?\.\(\)\)/);
});
