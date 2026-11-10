// V110-F92 The recording ended by itself at the size cap (Rust sends reason "size").
// V110-A27 No stop at 500 MB: Rust warns once (reason "warn") and stops at 5 GB.

type RecordingEvent = { recording: boolean; reason?: string | null };

export function recordingStopText(e: RecordingEvent): string | null {
  return !e.recording && e.reason === 'size' ? 'Stopped at 5 GB.' : null;
}

/** The warning past 500 MB: still recording, with a Stop action in the toast. */
export function recordingWarnText(e: RecordingEvent): string | null {
  return e.recording && e.reason === 'warn' ? 'Recording is over 500 MB.' : null;
}
