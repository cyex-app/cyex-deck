// V110-F92 The recording ended by itself at the size cap (Rust sends reason "size").
// V110-A27 No stop at 1 GB: Rust warns once (reason "warn") and stops at 5 GB.

type RecordingEvent = { recording: boolean; reason?: string | null };

export function recordingStopText(e: RecordingEvent): string | null {
  if (e.recording) return null;
  // V110-N1 A write error (disk full, drive removed) ends the recording (Rust sends reason "write").
  if (e.reason === 'write') return 'Recording stopped: the file can’t be written.';
  return e.reason === 'size' ? 'Stopped at 5 GB.' : null;
}

/** The warning past 1 GB: still recording, with a Stop action in the toast. */
export function recordingWarnText(e: RecordingEvent): string | null {
  return e.recording && e.reason === 'warn' ? 'Recording is over 1 GB.' : null;
}
