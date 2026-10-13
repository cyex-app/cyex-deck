// Effects every window runs: theme, accent and chat font (with added font files, V101-10)
// follow the settings; emote size (E3), still pictures while unfocused (E2), streamer mode
// (U4), who wears what (1.37-1.39), plugins (S5), media keys (V101-29), our own menu in place
// of the browser's (V101-30), smooth scrolling (V103-11) and outside drops (V105-14), picture scale (V105-08).

import { applyAppearance } from '../design/theme';
import { applyUserFonts } from '../design/userFonts';
import { looks } from '../features/emotes/looks.svelte';
import { align } from '../features/emotes/phase';
import { useMediaSession } from '../features/player/media.svelte';
import { plugins } from '../features/plugins/plugins.svelte';
import { density } from '../lib/density.svelte';
import { focus } from '../lib/focus.svelte';
import { guardDrops } from '../lib/drops';
import { guardNativeMenu } from '../lib/nativeMenu';
import { settings } from '../lib/settings.svelte';
import { startSmoothScroll } from '../lib/smoothScroll';
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

  $effect(() => applyUserFonts(settings.v.general.fonts));

  $effect(() => {
    document.documentElement.style.setProperty('--emote-scale', String(settings.v.chat.emoteSize / 100));
  });

  $effect(() => {
    document.documentElement.classList.toggle('still', focus.still);
    // V105-08 Modifiers paused while unfocused resume in step.
    if (!focus.still) align(document);
  });

  $effect(() => guardNativeMenu());
  $effect(() => guardDrops());
  $effect(() => (settings.v.general.smoothScroll ? startSmoothScroll() : undefined));
  $effect(() => focus.start());
  $effect(() => density.start());
  $effect(() => streamer.start());
  $effect(() => looks.start());
  $effect(() => plugins.start());
  useMediaSession();
}
