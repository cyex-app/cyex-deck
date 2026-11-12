// V110-N6 Texts for Retry on the "couldn't be opened" start notice.

import type { Partition } from '../ipc/bindings/Partition';
import { partitionList } from './partitions.ts';

/** What Retry says after asking Rust again. `ok` when nothing is blocked any more. */
export function retryBlockedText(result: { recovered: Partition[]; blocked: Partition[] }): { ok: boolean; text: string } {
  if (!result.blocked.length) return { ok: true, text: 'Opened again.' };
  return {
    ok: false,
    text: `Still can’t open ${partitionList(result.blocked)}. Close whatever is using the file and try again.`,
  };
}
