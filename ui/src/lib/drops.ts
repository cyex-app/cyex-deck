// V105-14 A file or link dropped on a window would open in place of the app. Drops the
// app doesn't handle are refused; text dropped into a text field still lands there.

/** True when the browser should handle this drop itself: text into a text field. */
export function letThrough(types: readonly string[], editable: boolean): boolean {
  return editable && !types.includes('Files');
}

export function guardDrops(): () => void {
  const editable = (t: EventTarget | null) =>
    t instanceof Element && !!t.closest('input, textarea, [contenteditable=""], [contenteditable="true"]');
  const over = (e: DragEvent) => {
    if (e.defaultPrevented || letThrough([...(e.dataTransfer?.types ?? [])], editable(e.target))) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'none';
  };
  const drop = (e: DragEvent) => {
    if (e.defaultPrevented || letThrough([...(e.dataTransfer?.types ?? [])], editable(e.target))) return;
    e.preventDefault();
  };
  window.addEventListener('dragover', over);
  window.addEventListener('drop', drop);
  return () => {
    window.removeEventListener('dragover', over);
    window.removeEventListener('drop', drop);
  };
}
