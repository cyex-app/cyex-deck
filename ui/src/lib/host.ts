/** V108-F34 A url's hostname without a leading `www.`, or '' for text that isn't a url. */
export function hostOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return '';
  }
}
