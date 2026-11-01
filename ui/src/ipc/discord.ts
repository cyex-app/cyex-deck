import { invoke } from '@tauri-apps/api/core';

import type { DiscordStatus } from './bindings/DiscordStatus';
import type { Watch } from './bindings/Watch';

export const discordIpc = {
  /** V108-N26 Whether this build has a Discord app ID. */
  available: () => invoke<boolean>('discord_available'),
  /** The channel playing now, or null for nothing (clears the activity). Rust spaces the updates. */
  watch: (watch: Watch | null) => invoke<void>('discord_watch', { watch }),
  /** The setting turned on or off; on makes Rust check the link now. */
  probe: (on: boolean) => invoke<void>('discord_probe', { on }),
  /** How the link to Discord stands. */
  status: () => invoke<DiscordStatus>('discord_status'),
};
