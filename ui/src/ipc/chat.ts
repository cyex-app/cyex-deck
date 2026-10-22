import { Channel, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { ChannelCatalog } from './bindings/ChannelCatalog';
import type { Chatters } from './bindings/Chatters';
import type { ChatMessage } from './bindings/ChatMessage';
import type { ChatOpened } from './bindings/ChatOpened';
import type { ChatShown } from './bindings/ChatShown';
import type { GlobalCatalog } from './bindings/GlobalCatalog';
import type { ChatIdentity } from './bindings/ChatIdentity';
import type { Smilies } from './bindings/Smilies';
import type { EmoteRewardChoices } from './bindings/EmoteRewardChoices';
import type { Points } from './bindings/Points';
import type { Profile } from './bindings/Profile';
import type { RoomEvent } from './bindings/RoomEvent';
import type { LinkInfo } from './bindings/LinkInfo';
import type { MenuContext } from './bindings/MenuContext';
import type { UiAction } from './bindings/UiAction';

export const chatIpc = {
  /** Opens a view on a channel's chat. Events arrive in 50 ms batches until `close`. */
  /** `filters`: F3 filter ids the view shows messages through. `follows`: the mod view's
   * or the stream manager's own chat, not a regular one (V103-22). */
  open: (
    login: string,
    account: string | null,
    filters: string[],
    onEvents: (events: RoomEvent[]) => void,
    follows = false,
  ) => {
    const channel = new Channel<RoomEvent[]>();
    channel.onmessage = onEvents;
    return invoke<ChatOpened>('chat_open', { login, account, filters, follows, onEvent: channel });
  },
  close: (view: number) => invoke<void>('chat_close', { view }),
  /** V103-22 Where the regular chats show, by window, and which windows still load. */
  shown: () => invoke<ChatShown>('chat_shown'),
  onShown: (handler: (s: ChatShown) => void) => listen<ChatShown>('chat:shown', (e) => handler(e.payload)),
  /** Commands run in Rust (1.8); some ask the window to do something. */
  send: (view: number, text: string, replyTo?: string) => invoke<UiAction | null>('chat_send', { view, text, replyTo }),
  /** S1 A menu entry or usercard button. */
  menu: (view: number, command: string, context: MenuContext) =>
    invoke<UiAction | null>('chat_menu', { view, command, context }),
  /** V1041-01 Command names (no `/`) this view can use, for Tab completion. */
  commands: (view: number) => invoke<string[]>('chat_commands', { view }),
  /** V1051-04 Tab's candidates for an argument of /combo, /paint and /badge. */
  commandArgs: (view: number, command: string, sub: string | null) =>
    invoke<string[]>('chat_command_args', { view, command, sub }),
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
  /** V107-Mk Smiley style by Twitch's set name; answers with the read-back. */
  setSmilies: (account: string, name: string) => invoke<Smilies>('chat_set_smilies', { account, name }),
  userMessages: (channel: string, userId: string) =>
    invoke<ChatMessage[]>('chat_user_messages', { channel, userId }),
  /** X2 Where a link ends up after redirects, and 1.52 its preview. */
  resolveLink: (url: string) => invoke<LinkInfo | null>('chat_resolve_link', { url }),
  profile: (login?: string, id?: string) => invoke<Profile | null>('user_profile', { login, id }),
  openUsercard: (channel: string, login: string) => invoke<void>('window_open_usercard', { channel, login }),
};
