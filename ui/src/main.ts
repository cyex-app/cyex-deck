import { getCurrentWindow } from '@tauri-apps/api/window';
import { mount } from 'svelte';

import './design/tokens.css';
import './design/emotes.css';
import './design/fonts.css';
import { applyCachedTheme } from './design/theme';
import { askResume } from './features/tabs/resume.svelte';
import { tabs } from './features/tabs/tabs.svelte';
import { chatxIpc } from './ipc/chatx';
import { accounts } from './lib/accounts.svelte';
import { showCrashNote } from './lib/crashNote';
import { installFindBlock } from './lib/findKey';
import { settings } from './lib/settings.svelte';
import { currentWindow } from './lib/window.svelte';

const panelRoute = /^#\/panel\/(.+)$/.exec(location.hash)?.[1];
// U7 Overlays are see-through and always dark: light text with an outline reads on anything.
if (panelRoute?.startsWith('overlay')) document.documentElement.classList.add('overlay');

applyCachedTheme();
installFindBlock();
// Each window loads only its own view (a pop-out or overlay doesn't parse the main window's),
// fetched while the settings load.
const panelView = panelRoute ? import('./app/PanelApp.svelte') : null;
const mainView = panelRoute ? null : import('./app/App.svelte');
async function start(): Promise<void> {
  await Promise.all([settings.load(), currentWindow.init(), accounts.load(), tabs.load()]);

  const target = document.getElementById('app')!;

  // C17 Translations go to the Windows language unless Settings says otherwise.
  if (!panelRoute) chatxIpc.setLanguage(navigator.language).catch(() => {});
  // V107-R12 Once after a crash, in the main window.
  if (!panelRoute && currentWindow.isMain) {
    void showCrashNote();
    // V108-N21 And which tabs to bring back, before any tab mounts.
    await askResume();
  }

  if (panelView) mount((await panelView).default, { target, props: { route: decodeURIComponent(panelRoute!) } });
  else mount((await mainView!).default, { target });
}

// A failed start (an IPC call refused, a view that won't load) says so instead of leaving
// the window empty.
start().catch((e: unknown) => {
  console.error('start', e);
  // The window has no title bar of its own yet, so the note brings its own Close.
  const box = document.createElement('div');
  box.className = 'start-failed';
  const note = document.createElement('p');
  note.textContent = 'cyex deck couldn’t start this window. Close it and try again.';
  const close = document.createElement('button');
  close.textContent = 'Close';
  close.onclick = () => void getCurrentWindow().close();
  box.append(note, close);
  (document.getElementById('app') ?? document.body).replaceChildren(box);
});
