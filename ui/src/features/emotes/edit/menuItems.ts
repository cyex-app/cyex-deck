// V106-31/32 The 7TV entries of an emote's right-click menu, from Rust's options. Pure.

import type { MenuEntry, MenuItem } from '../../../components/menuItems.ts';
import type { AddOptions } from '../../../ipc/bindings/AddOptions';
import type { AddTarget } from '../../../ipc/bindings/AddTarget';
import type { EditOptions } from '../../../ipc/bindings/EditOptions';
import type { HereEdit } from '../../../ipc/bindings/HereEdit';
import { blockedText } from './words.ts';

export interface EditHandlers {
  /** `ownName`: the emote's own 7TV name, what the add form starts with. */
  add: (t: AddTarget, ownName: string) => void;
  rename: (h: HereEdit) => void;
  remove: (h: HereEdit) => void;
}

/** U4 The greyed-out reason for the "add" entry as a whole (or its only target). */
export function addNote(add: AddOptions): string | undefined {
  if (add.blocked) return blockedText(add.blocked);
  const only = add.targets.length === 1 ? add.targets[0] : null;
  return only?.blocked ? blockedText(only.blocked, only.channel) : undefined;
}

/** U4 One item per channel the emote can be added to. */
export function targetItems(add: AddOptions, onpick: (t: AddTarget) => void): MenuItem[] {
  return add.targets.map((t) => ({
    label: t.channel,
    hint: t.viewed ? 'This chat' : t.own ? 'Yours' : undefined,
    disabled: !!t.blocked,
    note: t.blocked ? blockedText(t.blocked, t.channel) : undefined,
    run: () => onpick(t),
  }));
}

/** Nothing when there is nothing to offer (not ready, not 7TV, no channel). */
export function editEntries(o: EditOptions | null, h: EditHandlers): MenuEntry[] {
  if (!o || o.state !== 'ready') return [];
  const out: MenuItem[] = [];
  const add = o.add;
  if (add && add.targets.length > 0) {
    if (add.targets.length === 1) {
      const t = add.targets[0];
      out.push({
        label: `Add to ${t.channel}`,
        icon: 'seventv',
        disabled: !!(add.blocked ?? t.blocked),
        note: addNote(add),
        run: () => h.add(t, o.ownName),
      });
    } else {
      out.push({
        label: 'Add to 7TV channel',
        icon: 'seventv',
        disabled: !!add.blocked,
        note: addNote(add),
        items: targetItems(add, (t) => h.add(t, o.ownName)),
      });
    }
  }
  const here = o.here;
  if (here) {
    out.push(
      { label: 'Rename…', icon: 'pencil', note: `In ${here.channel}’s 7TV emotes`, run: () => h.rename(here) },
      { label: `Remove from ${here.channel}`, icon: 'trash', run: () => h.remove(here) },
    );
  }
  return out.length > 0 ? ['separator', ...out] : [];
}
