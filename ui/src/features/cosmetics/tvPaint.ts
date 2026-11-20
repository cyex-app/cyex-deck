// V112-16 A cyex.tv paint as inline CSS for a name, the simple way (Phase 4 draws chat):
// a gradient or picture clipped to the text, drop shadows, the fallback colour underneath.

import type { TvPaint } from '../../ipc/bindings/TvPaint';

const COLOR = /^#(?:[0-9a-f]{6}|[0-9a-f]{8})$/i;
const PICTURE = /^https:\/\/cdn\.cyex\.tv\/[\w./-]+$/;

/** The paint's background: a gradient, a picture, or null (fallback colour only). */
export function tvPaintImage(p: TvPaint): string | null {
  const rep = p.repeat ? 'repeating-' : '';
  if (p.kind === 'image') return p.image && PICTURE.test(p.image) ? `url("${p.image}")` : null;
  const stops = p.stops.filter((s) => COLOR.test(s.color));
  if (stops.length < 2) return null;
  const list = stops.map((s) => `${s.color} ${Math.round(s.at * 1000) / 10}%`).join(', ');
  return p.kind === 'radial'
    ? `${rep}radial-gradient(${p.shape === 'ellipse' ? 'ellipse' : 'circle'}, ${list})`
    : `${rep}linear-gradient(${p.angle}deg, ${list})`;
}

/** Declarations for `style=`: clipped to the text when there is a background. */
export function tvPaintStyle(p: TvPaint): string {
  const out: string[] = [];
  if (p.color && COLOR.test(p.color)) out.push(`color: ${p.color}`);
  const image = tvPaintImage(p);
  if (image) {
    out.push(`background-image: ${image}`);
    if (p.kind === 'image') out.push(p.repeat ? 'background-size: auto 100%' : 'background-size: cover');
    out.push('background-clip: text', '-webkit-background-clip: text', '-webkit-text-fill-color: transparent');
  }
  const shadows = p.shadows
    .filter((s) => COLOR.test(s.color))
    .slice(0, 4)
    .map((s) => `drop-shadow(${s.x}px ${s.y}px ${s.radius}px ${s.color})`);
  if (shadows.length) out.push(`filter: ${shadows.join(' ')}`);
  return out.join('; ');
}
