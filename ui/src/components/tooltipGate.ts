// rb-1 Whether a tip may open: something to say, its `when` holds, and no drag is on
// (a grip's tip would reopen on every row the pointer passes and cover the ghost).

export interface TipGateOptions {
  label: string;
  image?: string;
  when?: () => boolean;
}

export function tipAllowed(opts: TipGateOptions, dragging: boolean): boolean {
  if (dragging) return false;
  if (!opts.label && !opts.image) return false;
  return !opts.when || opts.when();
}
