// Panels that can pop out into their own window (1.51). A route is `<kind>` or
// `<kind>/<params>`; the kind picks the component. Usercards (1.36) open through
// `chatIpc.openUsercard`, not `popOut`, but render here too. Settings never pop out (V103-08).

import type { Component } from 'svelte';

import RecordingView from '../features/chat/RecordingView.svelte';
import RoomPanel from '../features/chat/RoomPanel.svelte';
import ModPanelWindow from '../features/modview/ModPanelWindow.svelte';
import { INFO, isModPanel } from '../features/modview/panelInfo';
import ModViewPanel from '../features/modview/ModViewPanel.svelte';
import OverlayView from '../features/overlay/OverlayView.svelte';
import ConsoleView from '../features/plugins/ConsoleView.svelte';
import { plugins } from '../features/plugins/plugins.svelte';
import SmPanelWindow from '../features/streammanager/SmPanelWindow.svelte';
import StreamManagerView from '../features/streammanager/StreamManagerView.svelte';
import ChatPanel from '../features/tabs/ChatPanel.svelte';
import TabPanel from '../features/tabs/TabPanel.svelte';
import { label, tabs, tiles } from '../features/tabs/tabs.svelte';
import UsercardView from '../features/usercard/UsercardView.svelte';
import WhispersView from '../features/whispers/WhispersView.svelte';
import { windowsIpc } from '../ipc/windows';
import { toast } from '../lib/toast.svelte';
import { errorText } from '../lib/errors';

export interface PanelKind {
  title: string;
  width: number;
  height: number;
  /** V104-03 Closing docks it back into the main window (3.14, V102-16). */
  docks?: boolean;
  /** V107-R29 Where it docks back to, when that isn't the main window. */
  dockHint?: string;
  /** V105-18 `slim`: a pop-up (usercards, `#channel` chats) with the slim header; must
   * match `is_popup` in `app/src/windows/usercard.rs`. `none`: the panel draws its own
   * (U7 overlays). Default: the large pop-out frame (V104-03). */
  frame?: 'slim' | 'none';
  component: Component<{ popout?: boolean; route: string }>;
}

export const PANELS: Record<string, PanelKind> = {
  tab: { title: 'Tab', width: 420, height: 680, docks: true, component: TabPanel },
  chat: { title: 'Chat', width: 380, height: 680, docks: true, component: ChatPanel },
  room: { title: 'Chat', width: 380, height: 640, frame: 'slim', component: RoomPanel },
  usercard: { title: 'User', width: 380, height: 560, frame: 'slim', component: UsercardView },
  recording: { title: 'Recording', width: 400, height: 640, component: RecordingView },
  modview: { title: 'Mod view', width: 920, height: 680, docks: true, component: ModViewPanel },
  // V107-R29 One mod view panel, `modpanel/<login>/<panel>`; closing it docks it back into the mod view.
  modpanel: { title: 'Mod view', width: 400, height: 560, docks: true, dockHint: 'Puts it back into the mod view.', component: ModPanelWindow },
  streammanager: { title: 'Stream manager', width: 1000, height: 760, docks: true, component: StreamManagerView },
  // V107-R40 One stream manager panel, `smpanel/<panel>`.
  smpanel: { title: 'Stream manager', width: 400, height: 600, component: SmPanelWindow },
  whispers: { title: 'Whispers', width: 720, height: 560, docks: true, component: WhispersView },
  overlay: { title: 'Overlay', width: 340, height: 420, frame: 'none', component: OverlayView },
  plugin: { title: 'Plugin console', width: 620, height: 420, component: ConsoleView },
};

export function panelFor(route: string): PanelKind | undefined {
  return PANELS[route.split('/')[0]];
}

/** Window title for a route, e.g. the tab's name. */
export function titleFor(route: string): string {
  const [kind, a] = route.split('/');
  if (kind === 'tab' && a) {
    const tab = tabs.tab(a);
    if (tab) return label(tab);
  }
  if (kind === 'chat' && a) {
    const tile = tabs.list.flatMap(tiles).find((t) => t.id === a);
    if (tile) return `${tile.channel.name} chat`;
  }
  // V102-14 The channel: the name is on the card already.
  if (kind === 'usercard' && a) return `#${a}`;
  // V105-18 Pop-ups show the channel, like the usercard.
  if (kind === 'room' && a) return `#${a}`;
  if (kind === 'modview' && a) return `Mod view · ${a}`;
  if (kind === 'modpanel' && a) {
    const id = route.split('/')[2] ?? '';
    return `${isModPanel(id) ? INFO[id].title : 'Mod view'} · ${a}`;
  }
  if (kind === 'smpanel' && a) return SM_PANEL_TITLES[a] ?? 'Stream manager';
  if (kind === 'whispers' && a) return `Whispers · ${a}`;
  if (kind === 'plugin' && a) return `${plugins.view?.list.find((p) => p.id === a)?.info?.name ?? a} console`;
  if (kind === 'overlay' && a) {
    const tile = tabs.list.flatMap(tiles).find((t) => t.id === a);
    if (tile) return `${tile.channel.name} overlay`;
  }
  return PANELS[kind]?.title ?? 'cyex deck';
}

const SM_PANEL_TITLES: Record<string, string> = { followers: 'Followers', roles: 'My assigned roles', rolesmgr: 'Roles Manager', rewards: 'Rewards', clips: 'Clips', videos: 'Videos', modset: 'Moderation settings' };

export async function popOut(route: string, title?: string): Promise<boolean> {
  const panel = panelFor(route);
  if (!panel) return false;
  try {
    await windowsIpc.openPanel(route, title ?? panel.title, panel.width, panel.height);
    return true;
  } catch (e) {
    toast.error(errorText(e));
    return false;
  }
}
