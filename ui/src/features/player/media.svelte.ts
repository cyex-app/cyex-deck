// V101-29 Media keys and Windows' media controls. WebView2 lists the window there while a
// player makes sound; play, pause and seeking then act on the player used or opened last,
// like the player hotkeys (S2).

import { type PlayerControls, players } from './players.svelte';

/** Call from a window's root component. */
export function useMediaSession(): void {
  const session = 'mediaSession' in navigator ? navigator.mediaSession : null;
  if (!session) return;

  const on = (action: MediaSessionAction, run: ((p: PlayerControls, d: MediaSessionActionDetails) => void) | null) => {
    try {
      session.setActionHandler(action, run && ((d) => players.active && run(players.active, d)));
    } catch {
      // Not every action is known to every build.
    }
  };

  $effect(() => {
    on('play', (p) => p.play?.());
    on('pause', (p) => p.pause?.());
    on('stop', (p) => p.pause?.());
    return () => {
      for (const a of ['play', 'pause', 'stop', 'seekbackward', 'seekforward'] as const) on(a, null);
      session.metadata = null;
    };
  });

  $effect(() => {
    const p = players.active;
    const info = p?.media?.() ?? null;
    session.metadata = info
      ? new MediaMetadata({ title: info.title, artist: info.artist, artwork: info.artwork ? [{ src: info.artwork }] : [] })
      : null;
    // Live streams can't seek: Windows then shows no seek buttons.
    const seek = Boolean(p?.seekBy);
    on('seekbackward', seek ? (q, d) => q.seekBy?.(-(d.seekOffset ?? 10)) : null);
    on('seekforward', seek ? (q, d) => q.seekBy?.(d.seekOffset ?? 10) : null);
  });
}
