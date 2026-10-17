// V102-01 The quick switch for unlisted 7TV emotes (2.4): one setting for every chat,
// flipped between blur and show from an emote's menu or card, like logs.cyex.app's
// "Show all unlisted emotes". "Hide" is only set in Settings → Emotes.

import { settings } from '../../lib/settings.svelte';
import { toast } from '../../lib/toast.svelte';

export function unlistedBlurred(): boolean {
  return settings.v.chat.unlistedEmotes === 'blur';
}

export function flipUnlisted(): void {
  void settings.patch('chat', { unlistedEmotes: unlistedBlurred() ? 'show' : 'blur' });
}

export { unlistedPrompt } from './prompt';

export function showUnlisted(): void {
  void settings.patch('chat', { unlistedEmotes: 'show', unlistedChosen: true });
  toast.info('Unlisted emotes show now. Settings → Emotes changes it.', {
    action: { label: 'Undo', run: () => void settings.patch('chat', { unlistedEmotes: 'blur', unlistedChosen: false }) },
  });
}

export function keepBlurred(): void {
  void settings.patch('chat', { unlistedChosen: true });
  toast.info('Unlisted emotes stay blurred. Settings → Emotes changes it.');
}
