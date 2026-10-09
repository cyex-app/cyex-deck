// Panels that can pop out into their own window (1.51). A route is `<kind>` or
// `<kind>/<id>`; the kind picks the component. Later phases add chat, mod view, etc.

import type { Component } from 'svelte';

import SettingsView from '../features/settings/SettingsView.svelte';
import { windowsIpc } from '../ipc/windows';
import { toast } from '../lib/toast.svelte';

export interface PanelKind {
  title: string;
  width: number;
  height: number;
  component: Component<{ popout?: boolean }>;
}

export const PANELS: Record<string, PanelKind> = {
  settings: { title: 'Settings', width: 820, height: 620, component: SettingsView },
};

export function panelFor(route: string): PanelKind | undefined {
  return PANELS[route.split('/')[0]];
}

export async function popOut(route: string): Promise<boolean> {
  const panel = panelFor(route);
  if (!panel) return false;
  try {
    await windowsIpc.openPanel(route, panel.title, panel.width, panel.height);
    return true;
  } catch (e) {
    toast.error(String(e));
    return false;
  }
}
