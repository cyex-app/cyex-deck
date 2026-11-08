// V110-C66 A trusted link (Rust marked it `embed`) is asked for once it is on screen, so its
// hover preview opens at once. One observer for all of them, no margin: lines in view only.
// Other links wait for the hover (V108-F46). Off when previews are off or streamer mode hides them.

import { cardsShown, embeds } from './linkPreviews';

let seen: IntersectionObserver | null = null;
const urls = new WeakMap<Element, string>();

function observer(): IntersectionObserver {
  return (seen ??= new IntersectionObserver((entries) => {
    for (const e of entries) {
      if (!e.isIntersecting) continue;
      const url = urls.get(e.target);
      seen!.unobserve(e.target);
      if (url && cardsShown()) embeds.get(url).catch(() => {});
    }
  }));
}

/** Svelte action: `use:preloadLink={url}` on a trusted link's `<a>`; leave it off other links. */
export function preloadLink(node: HTMLElement, url: string) {
  const watch = (u: string) => {
    if (!u) return;
    urls.set(node, u);
    observer().observe(node);
  };
  watch(url);
  return {
    update(next: string) {
      seen?.unobserve(node);
      urls.delete(node);
      watch(next);
    },
    destroy() {
      urls.delete(node);
      seen?.unobserve(node);
    },
  };
}
