// X6 While a recording plays in this window, its pictures come from its own folder, so a
// replay works offline. Keys match what Rust saved: `emote:<provider>:<id>`,
// `badge:<set>/<version>`, `gif:<id>`.

const HOST = 'http://cyexhls.localhost/rec/';

let base = '';
let files: Record<string, string> = {};

export function useSaved(recording: string, assets: Record<string, string>): void {
  base = `${HOST}${encodeURIComponent(recording)}/`;
  files = assets;
}

/** The saved copy of a picture, if this window plays a recording that has it. */
export function saved(key: string): string | undefined {
  const file = files[key];
  return file ? base + encodeURIComponent(file) : undefined;
}

/** L1 The recording closed: its pictures are no longer used here (a later one sets its own). */
export function dropSaved(recording: string): void {
  if (base !== `${HOST}${encodeURIComponent(recording)}/`) return;
  base = '';
  files = {};
}
