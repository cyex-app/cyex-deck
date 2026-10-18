// V1051-01 Where streams, VODs, clips and stories play (Settings → Sound). '' follows
// Windows. Chromium lists speakers and allows the pick only while the page may use the
// microphone, so Rust grants that around each pick and takes it back right after
// (`soundIpc.permit`). The device is found by its Windows name. Devices are only looked up
// once a player needs them, since listing them starts WebView2's audio process.

import { soundIpc } from '../../ipc/sound';
import { settings } from '../../lib/settings.svelte';
import { inGraph } from './gain';
import { sinkFor } from './sink';

interface Sinkable {
  readonly sinkId: unknown;
  setSinkId(id: string): Promise<void>;
}

const out = $state({ id: '' });
/** The boost graphs' outputs (gain.ts): players of their own, not in any graph. */
const outputs = new Set<HTMLMediaElement>();
let used = false;
/** Picks one at a time, so the permission is held by one pick and given back before the next. */
let queue: Promise<unknown> = Promise.resolve();

/** Runs `job` after the ones before it, so a late pick can't land after a newer one. */
function serial<T>(job: () => Promise<T>): Promise<T> {
  const run = queue.then(job);
  queue = run.catch(() => {});
  return run;
}

/** Runs `job` with the microphone permission, which is taken back whatever happens. */
function permitted<T>(job: () => Promise<T>): Promise<T> {
  return serial(async () => {
    await soundIpc.permit(true);
    try {
      return await job();
    } finally {
      await soundIpc.permit(false).catch(() => {});
    }
  });
}

/** The newest lookup: an older one still listing devices must not overwrite its answer. */
let lookups = 0;

async function resolve(): Promise<void> {
  const mine = ++lookups;
  const want = settings.v.general?.sound?.streamsDevice;
  if (!want) {
    out.id = '';
    return;
  }
  if (!used) return;
  try {
    const id = sinkFor(await permitted(() => navigator.mediaDevices.enumerateDevices()), want.name);
    if (mine === lookups) out.id = id;
  } catch {
    if (mine === lookups) out.id = '';
  }
}

$effect.root(() => {
  $effect(() => {
    void settings.v.general?.sound?.streamsDevice?.name;
    void resolve();
  });
  $effect(() => {
    const id = out.id;
    for (const el of outputs) setSink(el as unknown as Sinkable, id);
  });
});

function setSink(target: Sinkable, id: string): void {
  if (target.sinkId === id) return;
  // The default needs no permission, but waits its turn: a device pick still on its way
  // would land after it. A pick that is no longer the wanted one is dropped.
  const job = () => (out.id !== id || target.sinkId === id ? Promise.resolve() : target.setSinkId(id));
  void (id ? permitted(job) : serial(job)).catch(() => {});
}

/** A player's media pipeline asks for its speaker when it starts (a new stream, a reopen, an
 *  ad switch). Without the permission then, Chromium gives it a null sink for good: it decodes
 *  and plays nothing, and no later pick brings it back. So the permission is also held while a
 *  player loads, until it can play. */
const guarded = new WeakSet<HTMLMediaElement>();
const loading = new Map<HTMLMediaElement, ReturnType<typeof setTimeout>>();
/** Longest a load holds the permission. */
const LOAD_HOLD = 15_000;

function guard(el: HTMLMediaElement): void {
  if (guarded.has(el)) return;
  guarded.add(el);
  el.addEventListener('loadstart', () => hold(el));
  for (const e of ['canplay', 'playing', 'error', 'abort']) el.addEventListener(e, () => release(el));
  // Already loading when it got here (a clip's src is in the markup).
  if (el.networkState === HTMLMediaElement.NETWORK_LOADING && el.readyState < HTMLMediaElement.HAVE_FUTURE_DATA) hold(el);
}

function hold(el: HTMLMediaElement): void {
  if (!out.id || loading.has(el)) return;
  loading.set(el, setTimeout(() => release(el), LOAD_HOLD));
  void soundIpc.permit(true).catch(() => {});
}

function release(el: HTMLMediaElement): void {
  const timer = loading.get(el);
  if (timer === undefined) return;
  clearTimeout(timer);
  loading.delete(el);
  void soundIpc.permit(false).catch(() => {});
}

/** The first player looks the device up (listing devices starts WebView2's audio process). */
function use(): void {
  if (used) return;
  used = true;
  // Listening for device changes starts the audio process too, so only now.
  navigator.mediaDevices?.addEventListener('devicechange', () => void resolve());
  void resolve();
}

/** Sends a player's sound to the streams' device. Call it in an effect: it re-runs on a change. */
export function applySink(target: HTMLMediaElement): void {
  use();
  guard(target);
  // V1051-01 Boosted ones play through the boost context, which follows the device itself.
  if (inGraph(target)) return;
  setSink(target as unknown as Sinkable, out.id);
}

/** The boost's output player (V105-10) follows the same device (the effect above on a change).
 *  Its context stays on the default: one moved to a picked device suspends whenever the
 *  permission is given back, which froze the picture. */
export function routeOutput(el: HTMLMediaElement): void {
  outputs.add(el);
  use();
  guard(el);
  setSink(el as unknown as Sinkable, out.id);
}
