// V110-F22 One IntersectionObserver for every link card: `nearScreen` calls back once, when
// the node comes within a screen (600 px) of the view, so a card is filled while it is still
// off screen. Cheaper than one observer per card on a busy chat.

let seen: IntersectionObserver | null = null;
const calls = new WeakMap<Element, () => void>();

function observer(): IntersectionObserver {
  return (seen ??= new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue;
        const call = calls.get(e.target);
        leaveScreen(e.target);
        call?.();
      }
    },
    { rootMargin: '600px 0px' },
  ));
}

export function nearScreen(node: Element, call: () => void): void {
  calls.set(node, call);
  observer().observe(node);
}

/** Stops watching (also after the call came). */
export function leaveScreen(node: Element): void {
  calls.delete(node);
  seen?.unobserve(node);
}
