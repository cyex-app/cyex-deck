import { mount } from 'svelte';

import './design/tokens.css';
import App from './app/App.svelte';
import PanelApp from './app/PanelApp.svelte';
import { applyCachedTheme } from './design/theme';
import { tabs } from './features/tabs/tabs.svelte';
import { accounts } from './lib/accounts.svelte';
import { settings } from './lib/settings.svelte';
import { currentWindow } from './lib/window.svelte';

applyCachedTheme();
await Promise.all([settings.load(), currentWindow.init(), accounts.load(), tabs.load()]);

const target = document.getElementById('app')!;
const panelRoute = /^#\/panel\/(.+)$/.exec(location.hash)?.[1];

export default panelRoute
  ? mount(PanelApp, { target, props: { route: decodeURIComponent(panelRoute) } })
  : mount(App, { target });
