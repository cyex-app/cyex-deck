/** A scrollbar that shows on hover or while scrolling (pair with the `.quiet-scroll` class). */
export function quietScroll(node: HTMLElement) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const onscroll = () => {
    node.classList.add('scrolling');
    clearTimeout(timer);
    timer = setTimeout(() => node.classList.remove('scrolling'), 800);
  };
  node.classList.add('quiet-scroll');
  node.addEventListener('scroll', onscroll, { passive: true });
  return {
    destroy() {
      clearTimeout(timer);
      node.removeEventListener('scroll', onscroll);
    },
  };
}
