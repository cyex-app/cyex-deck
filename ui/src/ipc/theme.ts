import { invoke } from '@tauri-apps/api/core';

import type { AccentTokens } from './bindings/AccentTokens';
import type { Theme } from './bindings/Theme';

export const themeIpc = {
  accentTokens: (accent: string, theme: Theme) => invoke<AccentTokens>('theme_accent_tokens', { accent, theme }),
};
