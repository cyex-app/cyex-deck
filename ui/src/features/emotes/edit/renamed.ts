// c-1 V106-32 The emote the card shows after an edit made from it.
import type { EditOptions } from '../../../ipc/bindings/EditOptions';
import type { EditForm } from './run.ts';

/** A rename gives the shown emote its new name, so the card's heading, its lookups and the edit
 *  row follow it; an add leaves it as it was (same object). */
export function afterEdit<T extends { name: string }>(emote: T, form: EditForm, alias: string): T {
  return form.kind === 'rename' && alias !== emote.name ? { ...emote, name: alias } : emote;
}

/** rc-2 The emote under its name in this chat's set, when the card was opened from a line that
 *  still has an old name; else the same object. */
export function currentName<T extends { name: string }>(emote: T, options: EditOptions | null): T {
  const alias = options?.state === 'ready' ? options.here?.alias : undefined;
  return alias && alias !== emote.name ? { ...emote, name: alias } : emote;
}
