/** V108-F58 C1 Tells `onchange` whether a scrolling box has more below its bottom edge:
 *  on scroll, on resize and when its content changes (for a fade cue). */
export function moreBelow(node: HTMLElement, onchange: (more: boolean) => void) {
  let last: boolean | null = null;
  let raf = 0;
  const check = () => {
    raf = 0;
    const more = node.scrollHeight - node.scrollTop - node.clientHeight > 4;
    if (more !== last) onchange((last = more));
  };
  const queue = () => {
    if (!raf) raf = requestAnimationFrame(check);
  };
  node.addEventListener('scroll', queue, { passive: true });
  const size = new ResizeObserver(queue);
  size.observe(node);
  const content = new MutationObserver(queue);
  content.observe(node, { childList: true, subtree: true, characterData: true });
  queue();
  return {
    destroy() {
      if (raf) cancelAnimationFrame(raf);
      node.removeEventListener('scroll', queue);
      size.disconnect();
      content.disconnect();
    },
  };
}
