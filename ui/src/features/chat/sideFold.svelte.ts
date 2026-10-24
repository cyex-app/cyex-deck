// The message box row folds its side buttons into a "…" menu when narrow (see sideFold.ts).
// The buttons stay mounted: folded, they sit hidden under the "…" button and its menu clicks them.

import { menu, openMenuAt } from '../../components/menu.svelte';
import type { IconName } from '../../components/icons';
import { shouldFold } from './sideFold';

const ICONS: Record<string, IconName> = { points: 'points', bits: 'gem', identity: 'badge', emotes: 'smile' };

export class SideFold {
  folded = $state(false);
  /** Holds the buttons while folded. */
  stash = $state<HTMLElement>();
  #opened = $state(false);

  constructor(
    private row: () => HTMLElement | undefined,
    private send: () => boolean,
  ) {
    $effect(() => {
      if (!menu.open) this.#opened = false;
    });
  }

  /** The "…" menu is on screen. */
  get expanded(): boolean {
    return this.#opened && menu.open;
  }

  measure = () => {
    const row = this.row();
    if (!row) return;
    const outside = [...row.querySelectorAll<HTMLElement>('[data-out]')].filter((s) => {
      const first = s.firstElementChild as HTMLElement | null;
      return !first || !first.hidden;
    }).length;
    this.folded = shouldFold(row.clientWidth, outside, this.send());
  };

  /** Watches the row's width and its buttons; call from an effect. */
  watch(): (() => void) | undefined {
    const row = this.row();
    if (!row) return;
    this.measure();
    const size = new ResizeObserver(this.measure);
    size.observe(row);
    const kids = new MutationObserver(this.measure);
    kids.observe(row, { childList: true, subtree: true, attributes: true, attributeFilter: ['hidden'] });
    return () => {
      size.disconnect();
      kids.disconnect();
    };
  }

  /** One line per side button that shows right now; a pick clicks that button. */
  open = (e: MouseEvent) => {
    const items = [...(this.stash?.querySelectorAll<HTMLElement>('[data-side]') ?? [])].flatMap((slot) => {
      const root = slot.firstElementChild as HTMLElement | null;
      const trigger = root && !root.hidden ? root.querySelector<HTMLButtonElement>(':scope > button') : null;
      const key = slot.dataset.side ?? '';
      if (!trigger) return [];
      return [{ label: trigger.getAttribute('aria-label') ?? key, icon: ICONS[key], run: () => requestAnimationFrame(() => trigger.click()) }];
    });
    if (!items.length) return;
    this.#opened = true;
    openMenuAt(e, items);
  };
}
