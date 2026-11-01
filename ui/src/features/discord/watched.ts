// V108-N26 Which channel Discord shows: the stream with the sound, else the first one on air.

export interface WatchTile {
  id: string;
  channelId: string;
}

export interface ChannelLook {
  login: string;
  name: string;
  avatar: string | null;
}

/**
 * `players` are the tab's stream tiles, `live` the ones on air, `heard` the tile with the sound
 * (null when every tile keeps its own). Returns the tile to show, or null for nothing on air.
 */
export function watchedTile(players: readonly WatchTile[], live: readonly string[], heard: string | null): WatchTile | null {
  const onAir = players.filter((p) => live.includes(p.id));
  return onAir.find((p) => p.id === heard) ?? onAir[0] ?? null;
}

/** What goes to Rust: Twitch's picture only when it is a web address. */
export function watchOf(look: ChannelLook): ChannelLook {
  const name = look.name.trim() || look.login;
  return { login: look.login.toLowerCase(), name, avatar: look.avatar?.startsWith('https://') ? look.avatar : null };
}
