// Columns of a rule list editor (2.6, F2, F3, S4): which key, and how to edit it.

export interface Field<T> {
  key: keyof T & string;
  /** Checkbox text, tooltip and accessible name. */
  label: string;
  /** V105-16 Shorter checkbox text, so a row fits one line; `label` stays the tooltip. */
  short?: string;
  /** V105-17 A sound file: an upload button, then its name with play and ×. */
  kind: 'text' | 'area' | 'check' | 'color' | 'number' | 'select' | 'list' | 'sound';
  placeholder?: string;
  /** V105-16 Longer tooltip of a text field, when the label isn't enough. */
  hint?: string;
  options?: readonly { value: string; label: string }[];
  /** CSS width; text fields grow when unset. */
  width?: string;
  /** V105-18 Grid layout: column (1 to 11) and how many it spans. */
  col?: number;
  span?: number;
  /** V105-18 The flexible column: what the rule looks for. */
  main?: boolean;
  /** V105-18 Starts the second line when the row doesn't fit on one; this and all after. */
  alert?: boolean;
  /** Monospace text, e.g. a regex. */
  mono?: boolean;
  /** Only shown while this holds, e.g. the regex flavor. */
  when?: (item: T) => boolean;
}

/** `#rrggbbaa` from a picked `#rrggbb`, keeping the alpha it had (or 50%). */
export function withAlpha(picked: string, old: string): string {
  const alpha = old.length === 9 ? old.slice(7) : '7f';
  return `${picked}${alpha}`;
}

/** "a, b c" → ["a", "b", "c"]. */
export function splitList(text: string): string[] {
  return text
    .split(/[\s,]+/)
    .map((s) => s.trim())
    .filter(Boolean);
}
