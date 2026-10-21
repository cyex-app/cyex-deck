// 1.46 The mod view's panels: name, what it's for, least size, height when stacked, and
// V107-R29 the dock icon of the ones that have one (not the stream, chat or AutoMod queue).

import type { IconName } from '../../components/icons';
import type { Box } from '../../lib/panels';

export type ModPanelId = 'chat' | 'activity' | 'actions' | 'automod' | 'unban' | 'chatters' | 'mass' | 'stream' | 'terms' | 'rewards';

export interface ModPanelInfo {
  title: string;
  hint: string;
  min: Box;
  tall: number;
  icon?: IconName;
}

export const INFO: Record<ModPanelId, ModPanelInfo> = {
  stream: { title: 'Stream', hint: "The channel’s stream.", min: { w: 240, h: 170 }, tall: 280 },
  chat: { title: 'Chat', hint: 'The chat, as you.', min: { w: 240, h: 200 }, tall: 440 },
  activity: { title: 'Activity', hint: 'Follows, unban requests, watched users.', min: { w: 200, h: 110 }, tall: 240, icon: 'activity' },
  actions: { title: 'Mod actions', hint: 'Bans, timeouts, deletes and more.', min: { w: 240, h: 140 }, tall: 300, icon: 'gavel' },
  automod: { title: 'AutoMod queue', hint: 'Held messages to allow or deny.', min: { w: 200, h: 110 }, tall: 240 },
  unban: { title: 'Unban requests', hint: 'Who asks to be unbanned.', min: { w: 200, h: 110 }, tall: 220, icon: 'undo' },
  chatters: { title: 'Viewers', hint: "Who’s in the chat.", min: { w: 200, h: 180 }, tall: 280, icon: 'users' },
  // V107-R31
  terms: { title: 'Blocked terms', hint: 'Words and phrases chat can’t use.', min: { w: 220, h: 160 }, tall: 280, icon: 'filter' },
  // V107-R43
  rewards: { title: 'Reward requests', hint: 'Channel points redemptions to complete or refund.', min: { w: 240, h: 160 }, tall: 300, icon: 'points' },
  mass: { title: 'Mass actions', hint: 'Ban or time out many at once.', min: { w: 220, h: 140 }, tall: 260, icon: 'zap' },
};

export const isModPanel = (id: string): id is ModPanelId => id in INFO;
