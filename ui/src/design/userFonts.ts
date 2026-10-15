// V101-10 Font files the user added: one @font-face each, served by the app (`cyexfont://`).
// Names and files are cleaned in Rust (no quotes or braces; plain `<id>.<ext>` files).

import type { CustomFont } from '../ipc/bindings/CustomFont';

export function userFontCss(fonts: CustomFont[]): string {
  return fonts
    .filter((f) => /^[a-z0-9]+\.(ttf|otf|woff2?)$/.test(f.file) && !/["'\;{}<>]/.test(f.name))
    .map((f) => `@font-face { font-family: "${f.name}"; src: url("http://cyexfont.localhost/${f.file}"); font-display: swap; }`)
    .join('\n');
}

/** Writes the rules into the window's own style element. */
export function applyUserFonts(fonts: CustomFont[]): void {
  let el = document.getElementById('cyex-user-fonts');
  if (!el) {
    el = document.createElement('style');
    el.id = 'cyex-user-fonts';
    document.head.append(el);
  }
  el.textContent = userFontCss(fonts);
}
