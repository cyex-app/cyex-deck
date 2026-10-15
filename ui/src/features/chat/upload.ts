// V105-11 Pure parts of image uploads: hosts, file types.

export const IMAGE_EXTS = ['png', 'jpg', 'jpeg', 'gif', 'webp'];

/** `127.0.0.1:8080`, `i.nuuls.com`; empty when the text isn't a URL. */
export function hostOf(url: string): string {
  try {
    return new URL(url).host.toLowerCase();
  } catch {
    return '';
  }
}

/** The paths that are images, by extension. */
export function imagePaths(paths: string[]): string[] {
  return paths.filter((p) => IMAGE_EXTS.includes(p.split('.').pop()?.toLowerCase() ?? ''));
}

/** The file extension for a clipboard image's type; empty for anything else. */
export function extOfMime(type: string): string {
  return { 'image/png': 'png', 'image/jpeg': 'jpg', 'image/gif': 'gif', 'image/webp': 'webp' }[type.toLowerCase()] ?? '';
}

/** The links in our own "Uploaded <link> · delete: <link>" system line; null for any other text. */
export function uploadLine(text: string): { link: string; deletion: string } | null {
  const m = /^Uploaded (\S+)(?: · delete: (\S+))?$/.exec(text);
  return m ? { link: m[1], deletion: m[2] ?? '' } : null;
}
