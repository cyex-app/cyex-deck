// V110-N5 R2-5 Which row a menu key lands on. Pure: `at` is the focused row (-1: none), `count` the rows.
export function menuMoveTarget(at: number, count: number, key: string): number {
  if (count <= 0) return -1;
  if (key === 'Home') return 0;
  if (key === 'End') return count - 1;
  const step = key === 'ArrowDown' ? 1 : key === 'ArrowUp' ? -1 : 0;
  if (at < 0) return step > 0 ? 0 : count - 1;
  return Math.min(Math.max(at + step, 0), count - 1);
}
