// Effects every window runs: theme, accent and chat font follow the settings.

import { applyAppearance } from '../design/theme';
import { settings } from '../lib/settings.svelte';

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
}
