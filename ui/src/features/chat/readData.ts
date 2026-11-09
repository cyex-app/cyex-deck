// V110-F84 A `data-*` attribute that holds JSON, read in a delegated click handler. A broken
// value is skipped instead of throwing in the middle of the click.

/** The parsed value, or undefined when the text is missing or isn't JSON. */
export function readData<T>(text: string | undefined): T | undefined {
  if (!text) return undefined;
  try {
    return JSON.parse(text) as T;
  } catch {
    return undefined;
  }
}
