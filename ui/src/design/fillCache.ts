// V106 ra-3 One lookup per accent and base for theme previews; a failed one falls back to the
// raw accent and is asked again next time.

import type { Theme } from '../ipc/bindings/Theme';

export function fillCache(get: (accent: string, theme: Theme) => Promise<string>) {
  const fills = new Map<string, Promise<string>>();
  return (accent: string, theme: Theme): Promise<string> => {
    const key = `${theme}|${accent}`;
    let fill = fills.get(key);
    if (!fill) {
      fill = get(accent, theme).catch(() => {
        fills.delete(key);
        return accent;
      });
      fills.set(key, fill);
    }
    return fill;
  };
}
