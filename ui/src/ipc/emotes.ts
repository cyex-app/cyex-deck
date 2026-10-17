import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { Avatar } from './bindings/Avatar';
import type { BotCommand } from './bindings/BotCommand';
import type { EditDone } from './bindings/EditDone';
import type { EditOp } from './bindings/EditOp';
import type { EditOptions } from './bindings/EditOptions';
import type { EmoteCard } from './bindings/EmoteCard';
import type { EmoteMenu } from './bindings/EmoteMenu';
import type { Looks } from './bindings/Looks';
import type { NameCheck } from './bindings/NameCheck';
import type { Provider } from './bindings/Provider';
import type { Reachability } from './bindings/Reachability';
import type { RoomBadges } from './bindings/RoomBadges';

export const emotesIpc = {
  /** Who wears what among these users (1.37-1.39). */
  looks: (users: string[]) => invoke<Looks>('emotes_looks', { users }),
  /** Looks that changed, a quarter second after. */
  onLooks: (cb: (looks: Looks) => void) => listen<Looks>('emotes:looks', (e) => cb(e.payload)),
  /** A channel's emotes finished loading. */
  onLoaded: (cb: (login: string) => void) => listen<string>('emotes:loaded', (e) => cb(e.payload)),
  /** 1.40 */
  menu: (login: string, account: string | null) => invoke<EmoteMenu>('emotes_menu', { login, account }),
  /** 1.41 */
  card: (login: string, provider: Provider, id: string, name: string) =>
    invoke<EmoteCard>('emotes_card', { login, provider, id, name }),
  /** 1.39 FFZ's mod and VIP pictures. */
  roomBadges: (login: string) => invoke<RoomBadges>('emotes_room_badges', { login }),
  /** C13 */
  botCommands: (login: string) => invoke<BotCommand[]>('emotes_bot_commands', { login }),
  /** C23 */
  avatar: (login: string) => invoke<Avatar | null>('emotes_avatar', { login }),
  /** V106-31/32 Reads what the account can edit on 7TV. */
  editWarm: (account: string, fresh = false) => invoke<Reachability>('emotes_edit_warm', { account, fresh }),
  /** V106-31/32 What to offer for one emote. No network. */
  editOptions: (account: string, login: string, emoteId: string, name: string) =>
    invoke<EditOptions>('emotes_edit_options', { account, login, emoteId, name }),
  /** V106-31/32 Whether a name is free in a set. */
  editName: (account: string, setId: string, emoteId: string, alias: string, renameFrom: string | null = null) =>
    invoke<NameCheck>('emotes_edit_name', { account, setId, emoteId, alias, renameFrom }),
  /** V106-31/32 Rejects with an EditFail. */
  edit: (account: string, op: EditOp) => invoke<EditDone>('emotes_edit', { account, op }),
};
