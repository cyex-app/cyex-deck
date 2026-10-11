import { mount } from 'svelte';

import './design/tokens.css';
import './design/emotes.css';
import './design/fonts.css';
import App from './app/App.svelte';
import PanelApp from './app/PanelApp.svelte';
import { applyCachedTheme } from './design/theme';
import { tabs } from './features/tabs/tabs.svelte';
import { chatxIpc } from './ipc/chatx';
import { accounts } from './lib/accounts.svelte';
import { settings } from './lib/settings.svelte';
import { currentWindow } from './lib/window.svelte';

const panelRoute = /^#\/panel\/(.+)$/.exec(location.hash)?.[1];
// U7 Overlays are see-through and always dark: light text with an outline reads on anything.
if (panelRoute?.startsWith('overlay')) document.documentElement.classList.add('overlay');

applyCachedTheme();
await Promise.all([settings.load(), currentWindow.init(), accounts.load(), tabs.load()]);

const target = document.getElementById('app')!;

// C17 Translations go to the Windows language unless Settings says otherwise.
if (!panelRoute) chatxIpc.setLanguage(navigator.language).catch(() => {});

export default panelRoute
  ? mount(PanelApp, { target, props: { route: decodeURIComponent(panelRoute) } })
  : mount(App, { target });
