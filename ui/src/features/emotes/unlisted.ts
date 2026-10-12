// V102-01 The quick switch for unlisted 7TV emotes (2.4): one setting for every chat,
// flipped between blur and show from an emote's menu or card, like logs.cyex.app's
// "Show all unlisted emotes". "Hide" is only set in Settings → Emotes.

import { settings } from '../../lib/settings.svelte';

export function unlistedBlurred(): boolean {
  return settings.v.chat.unlistedEmotes === 'blur';
}

export function flipUnlisted(): void {
  void settings.patch('chat', { unlistedEmotes: unlistedBlurred() ? 'show' : 'blur' });
}
