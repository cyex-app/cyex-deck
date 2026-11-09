// V110-U55 The one quiet line under Backups: when the last copy was made, or why it fails.

import type { BackupLine } from '../../ipc/settings';

export interface BackupText {
  text: string;
  failing: boolean;
}

/** `when` formats a Date as the user's date and time. Nothing is said before the first copy. */
export function backupText(line: BackupLine | null, when: (d: Date) => string): BackupText | null {
  if (!line) return null;
  if (line.error) {
    const why = line.error.replace(/[.\s]+$/, '');
    return { text: `Backups are failing: ${why}. Trying again every hour.`, failing: true };
  }
  if (line.lastOk === null) return null;
  return { text: `Last backup ${when(new Date(line.lastOk * 1000))}`, failing: false };
}
