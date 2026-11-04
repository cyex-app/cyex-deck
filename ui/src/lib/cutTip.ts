import { tooltip } from '../components/tooltip.svelte';

/** V108-F58 C11 A tooltip with the full text, only while the text is cut off by its box. */
export function cutTip(node: HTMLElement, label: string) {
  const make = (l: string) => ({ label: l, when: () => node.scrollWidth > node.clientWidth });
  const tip = tooltip(node, make(label));
  return {
    update(next: string) {
      tip.update?.(make(next));
    },
    destroy() {
      tip.destroy?.();
    },
  };
}
