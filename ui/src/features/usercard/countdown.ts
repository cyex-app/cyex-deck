// V107-R24 Time left of a timeout on the usercard: "4:32 left", "1 h 12 min left".

/** `null` once `untilMs` has passed. */
export function timeLeft(untilMs: number, now: number): string | null {
  const secs = Math.ceil((untilMs - now) / 1000);
  if (secs <= 0) return null;
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d) return `${d} d${h ? ` ${h} h` : ''} left`;
  if (h) return `${h} h${m ? ` ${m} min` : ''} left`;
  return `${m}:${String(secs % 60).padStart(2, '0')} left`;
}
