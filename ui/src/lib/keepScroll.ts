// Views that unmount when you leave them (Home, Discover, Settings) remember where they were
// scrolled for the rest of the session. The list often arrives after the view is back, so the
// saved spot is retried as the content grows, until it's reached, the user scrolls, or 3 s pass.
//
//   <div class="scroll" use:keepScroll={'home'}>

const saved = new Map<string, number>();
const GIVE_UP_MS = 3000;

export function keepScroll(node: HTMLElement, key: string) {
  const target = saved.get(key) ?? 0;
  let restoring = target > 0;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const stop = () => {
    restoring = false;
    clearTimeout(timer);
    seen.disconnect();
  };

  const apply = () => {
    if (!restoring) return;
    node.scrollTop = target;
    if (Math.abs(node.scrollTop - target) < 2) stop();
  };

  const seen = new MutationObserver(apply);
  const resize = new ResizeObserver(apply);
  if (restoring) {
    seen.observe(node, { childList: true, subtree: true });
    resize.observe(node);
    for (const child of node.children) resize.observe(child);
    timer = setTimeout(stop, GIVE_UP_MS);
    requestAnimationFrame(apply);
  }

  const onScroll = () => {
    // Our own jumps while restoring don't count as the user's spot.
    if (!restoring) saved.set(key, node.scrollTop);
  };
  // The user took over: stop pulling the view back.
  const onUser = () => {
    if (restoring) stop();
  };
  node.addEventListener('scroll', onScroll, { passive: true });
  node.addEventListener('wheel', onUser, { passive: true });
  node.addEventListener('pointerdown', onUser, { passive: true });
  node.addEventListener('keydown', onUser);

  return {
    destroy() {
      stop();
      resize.disconnect();
      node.removeEventListener('scroll', onScroll);
      node.removeEventListener('wheel', onUser);
      node.removeEventListener('pointerdown', onUser);
      node.removeEventListener('keydown', onUser);
    },
  };
}

/** Back to the top the next time the view opens (and now, if it's open: set scrollTop too). */
export function forgetScroll(key: string): void {
  saved.delete(key);
}
