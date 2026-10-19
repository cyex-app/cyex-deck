// L1 A <video> or <audio> that goes away lets go of its stream at once: paused, no
// source, reloaded empty. Otherwise its decoder and buffers stay until the element is
// collected, which can be much later.

/** Svelte action: `<video use:releaseMedia>`. */
export function releaseMedia(el: HTMLMediaElement): { destroy(): void } {
  return {
    destroy() {
      try {
        el.pause();
        el.removeAttribute('src');
        el.srcObject = null;
        el.load();
      } catch {
        // Already torn down.
      }
    },
  };
}
