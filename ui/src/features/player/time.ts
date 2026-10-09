// Player clocks: "2:05", "1:02:05".

const pad = (n: number) => String(n).padStart(2, '0');

export function clock(seconds: number): string {
  const s = Math.max(0, Math.floor(Number.isFinite(seconds) ? seconds : 0));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return h ? `${h}:${pad(m)}:${pad(s % 60)}` : `${m}:${pad(s % 60)}`;
}

/** Uptime of a stream that started at `startedAt` (RFC 3339). */
export function uptime(startedAt: string, now = Date.now()): string {
  const t = Date.parse(startedAt);
  return Number.isNaN(t) ? '' : clock((now - t) / 1000);
}
