// V110-F92 The recording ended by itself at the size cap (Rust sends reason "size").

export function recordingStopText(e: { recording: boolean; reason?: string | null }): string | null {
  return !e.recording && e.reason === 'size' ? 'Recording stopped at 500 MB.' : null;
}
