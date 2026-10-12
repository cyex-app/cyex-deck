import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { Avatar } from './bindings/Avatar';
import type { BotCommand } from './bindings/BotCommand';
import type { EmoteCard } from './bindings/EmoteCard';
import type { EmoteMenu } from './bindings/EmoteMenu';
import type { Looks } from './bindings/Looks';
import type { Provider } from './bindings/Provider';
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
};
