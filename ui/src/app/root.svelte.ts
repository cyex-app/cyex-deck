// Effects every window runs: theme, accent and chat font follow the settings; emote size
// (E3), still pictures while unfocused (E2), streamer mode (U4), who wears what
// (1.37-1.39), plugins (S5) and media keys (V101-29).

import { applyAppearance } from '../design/theme';
import { looks } from '../features/emotes/looks.svelte';
import { useMediaSession } from '../features/player/media.svelte';
import { plugins } from '../features/plugins/plugins.svelte';
import { focus } from '../lib/focus.svelte';
import { settings } from '../lib/settings.svelte';
import { streamer } from '../lib/streamer.svelte';

export function useRootEffects(): void {
  $effect(() => {
    void applyAppearance(settings.v.appearance);
  });

  $effect(() => {
    const { chatFontFamily, chatFontSize } = settings.v.general;
    const style = document.documentElement.style;
    style.setProperty('--chat-font', chatFontFamily ? `"${chatFontFamily}", var(--font-ui)` : 'var(--font-ui)');
    style.setProperty('--chat-size', `${chatFontSize}px`);
  });

  $effect(() => {
    document.documentElement.style.setProperty('--emote-scale', String(settings.v.chat.emoteSize / 100));
  });

  $effect(() => {
    document.documentElement.classList.toggle('still', focus.still);
  });

  $effect(() => focus.start());
  $effect(() => streamer.start());
  $effect(() => looks.start());
  $effect(() => plugins.start());
  useMediaSession();
}
