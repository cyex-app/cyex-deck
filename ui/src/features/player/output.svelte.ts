// V1051-01 Where streams, VODs, clips and stories play (Settings → Sound). '' follows
// Windows. Rust lets the page pick while a device is set (sound:sink-ready); the device is
// found by its Windows name. Devices are only looked up once a player needs them, since
// listing them starts WebView2's audio process.

import { soundIpc } from '../../ipc/sound';
import { settings } from '../../lib/settings.svelte';
import { sinkFor } from './sink';

interface Sinkable {
  readonly sinkId: unknown;
  setSinkId(id: string): Promise<void>;
}

const out = $state({ id: '' });
const contexts = new Set<AudioContext>();
let used = false;

async function resolve(): Promise<void> {
  const want = settings.v.general.sound.streamsDevice;
  if (!want) {
    out.id = '';
    return;
  }
  if (!used) return;
  try {
    out.id = sinkFor(await navigator.mediaDevices.enumerateDevices(), want.name);
  } catch {
    out.id = '';
  }
}

$effect.root(() => {
  $effect(() => {
    void settings.v.general.sound.streamsDevice?.name;
    void resolve();
  });
  $effect(() => {
    const id = out.id;
    for (const ctx of contexts) setSink(ctx as unknown as Sinkable, id);
  });
});
navigator.mediaDevices?.addEventListener('devicechange', () => void resolve());
void soundIpc.onSinkReady(() => void resolve());

function setSink(target: Sinkable, id: string): void {
  if (target.sinkId !== id) void target.setSinkId(id).catch(() => {});
}

/** The first player looks the device up (listing devices starts WebView2's audio process). */
function use(): void {
  if (used) return;
  used = true;
  void resolve();
}

/** Sends a player's sound to the streams' device. Call it in an effect: it re-runs on a change. */
export function applySink(target: HTMLMediaElement): void {
  use();
  setSink(target as unknown as Sinkable, out.id);
}

/** The boost's context (V105-10) follows the same device (the effect above on a change). */
export function routeContext(ctx: AudioContext): void {
  contexts.add(ctx);
  use();
  setSink(ctx as unknown as Sinkable, out.id);
}
