// What a click on a player landed on (V101-27). `closest` alone climbs past the player
// into the app around it, which has its own `.top` and `.bar`: a click on the picture then
// counted as one on the controls, so neither click to pause nor double-click worked.

interface El {
  matches(selector: string): boolean;
  parentElement: El | null;
}

/** Whether `target` or an element between it and `root` matches `selector`. */
export function hitInside(target: El | null, root: El, selector: string): boolean {
  for (let el = target; el && el !== root; el = el.parentElement) {
    if (el.matches(selector)) return true;
  }
  return false;
}
