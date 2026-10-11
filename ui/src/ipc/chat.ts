import { Channel, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { ChannelCatalog } from './bindings/ChannelCatalog';
import type { Chatters } from './bindings/Chatters';
import type { ChatMessage } from './bindings/ChatMessage';
import type { ChatOpened } from './bindings/ChatOpened';
import type { GlobalCatalog } from './bindings/GlobalCatalog';
import type { Mention } from './bindings/Mention';
import type { ChatIdentity } from './bindings/ChatIdentity';
import type { EmoteRewardChoices } from './bindings/EmoteRewardChoices';
import type { Points } from './bindings/Points';
import type { Profile } from './bindings/Profile';
import type { RoomEvent } from './bindings/RoomEvent';
import type { LinkInfo } from './bindings/LinkInfo';
import type { MenuContext } from './bindings/MenuContext';
import type { UiAction } from './bindings/UiAction';

export const chatIpc = {
  /** Opens a view on a channel's chat. Events arrive in 50 ms batches until `close`. */
  /** `filters`: F3 filter ids the view shows messages through. */
  open: (login: string, account: string | null, filters: string[], onEvents: (events: RoomEvent[]) => void) => {
    const channel = new Channel<RoomEvent[]>();
    channel.onmessage = onEvents;
    return invoke<ChatOpened>('chat_open', { login, account, filters, onEvent: channel });
  },
  close: (view: number) => invoke<void>('chat_close', { view }),
  /** Commands run in Rust (1.8); some ask the window to do something. */
  send: (view: number, text: string, replyTo?: string) => invoke<UiAction | null>('chat_send', { view, text, replyTo }),
  /** S1 A menu entry or usercard button. */
  menu: (view: number, command: string, context: MenuContext) =>
    invoke<UiAction | null>('chat_menu', { view, command, context }),
  globalCatalog: () => invoke<GlobalCatalog>('chat_global_catalog'),
  channelCatalog: (login: string) => invoke<ChannelCatalog | null>('chat_channel_catalog', { login }),
  points: (view: number, fresh = true) => invoke<Points | null>('chat_points', { view, fresh }),
  /** 1.48, V101-35 Everyone when `account` moderates there, else Twitch's sample. */
  viewers: (account: string | null, channel: string) => invoke<Chatters>('chat_viewers', { account, channel }),
  /** Resolves with a short line to show, e.g. "Redeemed." */
  /** `emote` is the pick for the two emote rewards. */
  redeem: (view: number, rewardId: string, text?: string, emote?: string) =>
    invoke<string>('chat_redeem', { view, rewardId, text, emote }),
  /** What the "choose an emote" rewards can still give. */
  emoteChoices: (view: number) => invoke<EmoteRewardChoices>('chat_emote_choices', { view }),
  /** 1.40 Badge picks and name color in a channel. */
  identity: (account: string, login: string) => invoke<ChatIdentity>('chat_identity', { account, login }),
  /** `channelId` `null` = everywhere; `set` `null` takes the badge off. */
  pickBadge: (account: string, channelId: string | null, set: string | null, version: string | null) =>
    invoke<void>('chat_pick_badge', { account, channelId, set, version }),
  setColor: (account: string, color: string) => invoke<void>('chat_set_color', { account, color }),
  userMessages: (channel: string, userId: string) =>
    invoke<ChatMessage[]>('chat_user_messages', { channel, userId }),
  /** X2 Where a link ends up after redirects, and 1.52 its preview. */
  resolveLink: (url: string) => invoke<LinkInfo | null>('chat_resolve_link', { url }),
  profile: (login?: string, id?: string) => invoke<Profile | null>('user_profile', { login, id }),
  openUsercard: (channel: string, login: string) => invoke<void>('window_open_usercard', { channel, login }),
  /** U6: only the main window gets these. */
  onMention: (handler: (m: Mention) => void) => listen<Mention>('chat:mention', (e) => handler(e.payload)),
};
