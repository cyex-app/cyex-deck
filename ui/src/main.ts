import { mount } from 'svelte';

import './design/tokens.css';
import App from './app/App.svelte';
import PanelApp from './app/PanelApp.svelte';
import { applyCachedTheme } from './design/theme';
import { settings } from './lib/settings.svelte';
import { currentWindow } from './lib/window.svelte';

applyCachedTheme();
await Promise.all([settings.load(), currentWindow.init()]);

const target = document.getElementById('app')!;
const panelRoute = /^#\/panel\/(.+)$/.exec(location.hash)?.[1];

export default panelRoute
  ? mount(PanelApp, { target, props: { route: decodeURIComponent(panelRoute) } })
  : mount(App, { target });
