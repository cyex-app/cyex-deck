// V111-22 One place that decides which profile picture a spot shows: the user's 7TV one,
// or Twitch's when the 7TV one errors or isn't loaded in AVATAR_TIMEOUT_MS. Call during
// component init. A failure is remembered per login for the session, so other spots use
// Twitch at once; a late load never flips back.

import { AVATAR_TIMEOUT_MS, FailedSet, next, type Phase } from './avatarFallback';
import { avatars } from './avatars.svelte';

const memory = new FailedSet();
/** Bumped on a new failure so every spot re-reads the memory. */
let version = $state(0);

export function sevenTvPick(login: () => string | null | undefined) {
  const url = $derived(avatars.of(login()));
  const key = $derived(login()?.toLowerCase() ?? '');

  $effect(() => {
    const u = url;
    const k = key;
    if (!u || !k || memory.has(k)) return;
    let phase: Phase = 'trying';
    const step = (e: 'load' | 'error' | 'timeout') => {
      phase = next(phase, e);
      if (phase === 'failed' && !memory.has(k)) {
        memory.add(k);
        version++;
      }
    };
    const img = new Image();
    img.referrerPolicy = 'no-referrer';
    img.onload = () => step('load');
    img.onerror = () => step('error');
    img.src = u;
    const t = setTimeout(() => step('timeout'), AVATAR_TIMEOUT_MS);
    return () => {
      clearTimeout(t);
      img.onload = img.onerror = null;
    };
  });

  return {
    /** The 7TV picture's URL while it is the one to show, else `null`. */
    get url(): string | null {
      void version;
      return key && !memory.has(key) ? url : null;
    },
    /** It has one but couldn't use it. */
    get failed(): boolean {
      void version;
      return !!key && !!url && memory.has(key);
    },
  };
}
