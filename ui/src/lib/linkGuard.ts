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
    const href = a instanceof HTMLAnchorElement ? a.href : a.href.baseVal;
    const act = anchorAction(new URL(href, location.href).href, location.origin, e.defaultPrevented);
    if (act === 'ignore') return;
    e.preventDefault();
    if (act === 'open' && !(e.type === 'auxclick' && e.button === 2)) {
      systemIpc.openChatLink(new URL(href, location.href).href).catch((err) => toast.error(errorText(err)));
    }
  };
  window.addEventListener('click', on);
  window.addEventListener('auxclick', on);
  return () => {
    window.removeEventListener('click', on);
    window.removeEventListener('auxclick', on);
  };
}
