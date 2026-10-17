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
const contexts = new Set<AudioContext>();
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
    for (const ctx of contexts) setSink(ctx as unknown as Sinkable, id);
  });
});

function setSink(target: Sinkable, id: string): void {
  if (target.sinkId === id) return;
  // The default needs no permission, but waits its turn: a device pick still on its way
  // would land after it. A pick that is no longer the wanted one is dropped.
  const job = () => (out.id !== id || target.sinkId === id ? Promise.resolve() : target.setSinkId(id));
  void (id ? permitted(job) : serial(job)).catch(() => {});
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
  // V1051-01 Boosted ones play through the boost context, which follows the device itself.
  if (inGraph(target)) return;
  setSink(target as unknown as Sinkable, out.id);
}

/** The boost's context (V105-10) follows the same device (the effect above on a change). */
export function routeContext(ctx: AudioContext): void {
  contexts.add(ctx);
  use();
  setSink(ctx as unknown as Sinkable, out.id);
}
