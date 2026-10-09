// Panels that can pop out into their own window (1.51). A route is `<kind>` or
// `<kind>/<params>`; the kind picks the component. Usercards (1.36) open through
// `chatIpc.openUsercard`, not `popOut`, but render here too.

import type { Component } from 'svelte';

import ChatPanel from '../features/chat/ChatPanel.svelte';
import SettingsView from '../features/settings/SettingsView.svelte';
import UsercardView from '../features/usercard/UsercardView.svelte';
import { windowsIpc } from '../ipc/windows';
import { toast } from '../lib/toast.svelte';

export interface PanelKind {
  title: string;
  width: number;
  height: number;
  /** Small window: no minimize button. */
  compact?: boolean;
  component: Component<{ popout?: boolean; route: string }>;
}

export const PANELS: Record<string, PanelKind> = {
  settings: { title: 'Settings', width: 820, height: 620, component: SettingsView },
  chat: { title: 'Chat', width: 380, height: 640, component: ChatPanel },
  usercard: { title: 'User', width: 340, height: 480, compact: true, component: UsercardView },
};

export function panelFor(route: string): PanelKind | undefined {
  return PANELS[route.split('/')[0]];
}

/** Window title for a route, e.g. the channel name for a chat. */
export function titleFor(route: string): string {
  const [kind, a, b] = route.split('/');
  if (kind === 'chat' && a) return a;
  if (kind === 'usercard' && b) return b;
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
