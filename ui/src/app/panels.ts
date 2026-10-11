// Panels that can pop out into their own window (1.51). A route is `<kind>` or
// `<kind>/<params>`; the kind picks the component. Usercards (1.36) open through
// `chatIpc.openUsercard`, not `popOut`, but render here too.

import type { Component } from 'svelte';

import RecordingView from '../features/chat/RecordingView.svelte';
import ModViewPanel from '../features/modview/ModViewPanel.svelte';
import OverlayView from '../features/overlay/OverlayView.svelte';
import ConsoleView from '../features/plugins/ConsoleView.svelte';
import { plugins } from '../features/plugins/plugins.svelte';
import SettingsView from '../features/settings/SettingsView.svelte';
import StreamManagerView from '../features/streammanager/StreamManagerView.svelte';
import ChatPanel from '../features/tabs/ChatPanel.svelte';
import TabPanel from '../features/tabs/TabPanel.svelte';
import { label, tabs, tiles } from '../features/tabs/tabs.svelte';
import UsercardView from '../features/usercard/UsercardView.svelte';
import WhispersView from '../features/whispers/WhispersView.svelte';
import { windowsIpc } from '../ipc/windows';
import { toast } from '../lib/toast.svelte';

export interface PanelKind {
  title: string;
  width: number;
  height: number;
  /** Small window: no minimize button. */
  compact?: boolean;
  /** No title bar: the panel draws its own (U7 overlays). */
  bare?: boolean;
  component: Component<{ popout?: boolean; route: string }>;
}

export const PANELS: Record<string, PanelKind> = {
  settings: { title: 'Settings', width: 820, height: 620, component: SettingsView },
  tab: { title: 'Tab', width: 420, height: 680, component: TabPanel },
  chat: { title: 'Chat', width: 380, height: 680, component: ChatPanel },
  usercard: { title: 'User', width: 380, height: 560, compact: true, component: UsercardView },
  recording: { title: 'Recording', width: 400, height: 640, component: RecordingView },
  modview: { title: 'Mod view', width: 920, height: 680, component: ModViewPanel },
  streammanager: { title: 'Stream manager', width: 1000, height: 760, component: StreamManagerView },
  whispers: { title: 'Whispers', width: 720, height: 560, component: WhispersView },
  overlay: { title: 'Overlay', width: 340, height: 420, bare: true, component: OverlayView },
  plugin: { title: 'Plugin console', width: 620, height: 420, component: ConsoleView },
};

export function panelFor(route: string): PanelKind | undefined {
  return PANELS[route.split('/')[0]];
}

/** Window title for a route, e.g. the tab's name. */
export function titleFor(route: string): string {
  const [kind, a, b] = route.split('/');
  if (kind === 'tab' && a) {
    const tab = tabs.tab(a);
    if (tab) return label(tab);
  }
  if (kind === 'chat' && a) {
    const tile = tabs.list.flatMap(tiles).find((t) => t.id === a);
    if (tile) return `${tile.channel.name} chat`;
  }
  if (kind === 'usercard' && b) return b;
  if (kind === 'modview' && a) return `Mod view · ${a}`;
  if (kind === 'whispers' && a) return `Whispers · ${a}`;
  if (kind === 'plugin' && a) return `${plugins.view?.list.find((p) => p.id === a)?.info?.name ?? a} console`;
  if (kind === 'overlay' && a) {
    const tile = tabs.list.flatMap(tiles).find((t) => t.id === a);
    if (tile) return `${tile.channel.name} overlay`;
  }
  return PANELS[kind]?.title ?? 'cyex';
}

export async function popOut(route: string, title?: string): Promise<boolean> {
  const panel = panelFor(route);
  if (!panel) return false;
  try {
    await windowsIpc.openPanel(route, title ?? panel.title, panel.width, panel.height);
    return true;
  } catch (e) {
    toast.error(String(e));
    return false;
  }
}
