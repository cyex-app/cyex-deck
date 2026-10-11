// V101-26 In fullscreen the browser shows only the fullscreen element and what's inside it.
// Menus, tooltips and toasts move in there while it lasts, and back after.

export function followFullscreen(node: HTMLElement) {
  const home = document.createComment('layers');
  node.before(home);
  const place = () => {
    const full = document.fullscreenElement;
    if (full && !full.contains(node)) full.append(node);
    else if (!full && home.nextSibling !== node) home.after(node);
  };
  document.addEventListener('fullscreenchange', place);
  return {
    destroy() {
      document.removeEventListener('fullscreenchange', place);
      home.after(node);
      home.remove();
    },
  };
}
