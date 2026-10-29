// V108-N02 Saving a clip as an MP4: what the progress reads as.

import { formatBytes } from '../../lib/format.ts';

export interface SaveProgress {
  done: number;
  total: number | null;
}

/** Whole percent, null when the size isn't known (or nothing came yet). */
export function savePercent(p: SaveProgress): number | null {
  if (p.total === null || p.total <= 0) return null;
  return Math.min(100, Math.floor((p.done / p.total) * 100));
}

/** "Saving…", "Saving 42 %" or, without a size, "Saving 3,2 MB". */
export function saveText(p: SaveProgress | undefined): string {
  if (!p || p.done <= 0) return 'Saving…';
  const pct = savePercent(p);
  return pct === null ? `Saving ${formatBytes(p.done)}` : `Saving ${pct} %`;
}
