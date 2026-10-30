// C25 The lines that start a new day (local time). Run over the whole chat on every batch,
// so a date is only built when a line leaves the day before it.

type Timed = { id: string; ts: number };

/** `now` stands for "today": the first line gets a date too when it isn't from today. */
export function dayStarts(lines: readonly Timed[], now: number): Set<string> {
  const starts = new Set<string>();
  let [from, to] = dayOf(now);
  for (const m of lines) {
    if (m.ts >= from && m.ts < to) continue;
    starts.add(m.id);
    [from, to] = dayOf(m.ts);
  }
  return starts;
}

/** The local day around `ts`, as [start, end) in ms. */
function dayOf(ts: number): [number, number] {
  const d = new Date(ts);
  return [
    new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime(),
    new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1).getTime(),
  ];
}
