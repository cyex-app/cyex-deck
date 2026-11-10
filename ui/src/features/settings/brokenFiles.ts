// V110-A44 The "Restore from a broken file" list: one row per file a start set aside.

import type { BrokenFile } from '../../ipc/settings';
import { formatBytes } from '../../lib/format.ts';
import { PARTITION_LABELS } from '../../lib/partitions.ts';

/** "Tabs" and "2026/10/09 12:00 · 3 KB". */
export function brokenRow(file: BrokenFile): { label: string; hint: string } {
  return { label: PARTITION_LABELS[file.partition], hint: `${file.when} · ${formatBytes(file.size)}` };
}

/** The toast after a restore. */
export function brokenRestoredText(file: BrokenFile): string {
  return `Restored ${PARTITION_LABELS[file.partition].toLowerCase()} from the broken file.`;
}
