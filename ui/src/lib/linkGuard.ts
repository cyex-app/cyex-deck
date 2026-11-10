// V110-F83 Last resort for links nobody routed: a click on an `<a href>` must never make the
// WebView leave the app. Handlers that know the link run first; this only sees what is left.

import { systemIpc } from '../ipc/system';
import { anchorAction } from './anchorAction';
import { errorText } from './errors';
import { toast } from './toast.svelte';

export function guardLinks(): () => void {
  const on = (e: MouseEvent) => {
    const a = e.target instanceof Element ? e.target.closest('a[href]') : null;
    if (!(a instanceof HTMLAnchorElement) && !(a instanceof SVGAElement)) return;
    const raw = a instanceof HTMLAnchorElement ? a.href : a.href.baseVal;
    // V110-F99 An href that can't be read is stopped like any other stray link, not thrown on.
    let href = '';
    try {
      href = new URL(raw, location.href).href;
    } catch {
      /* stays '' : anchorAction stops it */
    }
    const act = anchorAction(href, location.origin, e.defaultPrevented);
    if (act === 'ignore') return;
    e.preventDefault();
    if (act === 'open' && !(e.type === 'auxclick' && e.button === 2)) {
      systemIpc.openChatLink(href).catch((err) => toast.error(errorText(err)));
    }
  };
  window.addEventListener('click', on);
  window.addEventListener('auxclick', on);
  return () => {
    window.removeEventListener('click', on);
    window.removeEventListener('auxclick', on);
  };
}
