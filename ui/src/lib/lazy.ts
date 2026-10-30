// A view loaded off the start path: fetched once the window is idle, or on first use if
// that comes sooner. The same promise every time, so a view shown again draws at once.

const IDLE_MS = 5000;

export function lazy<T>(load: () => Promise<T>): () => Promise<T> {
  let loading: Promise<T> | undefined;
  const get = () =>
    (loading ??= load().catch((e: unknown) => {
      loading = undefined; // tried again next time
      throw e;
    }));
  if (typeof requestIdleCallback === 'function') requestIdleCallback(() => void get().catch(() => {}), { timeout: IDLE_MS });
  return get;
}
