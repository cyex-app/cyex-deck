<!-- V1051-01 Settings → Sound: the app in Windows' volume mixer, and where streams and the
     other sounds play. Changes apply at once. -->
<script lang="ts">
  import IconButton from '../../../components/IconButton.svelte';
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { OutputDevice } from '../../../ipc/bindings/OutputDevice';
  import type { Sound } from '../../../ipc/bindings/Sound';
  import { soundIpc } from '../../../ipc/sound';
  import { settings } from '../../../lib/settings.svelte';

  const s = $derived(settings.v.general.sound);
  let devices = $state<OutputDevice[]>([]);
  /** Until the first list arrives, a saved device isn't called "not connected". */
  let listed = $state(false);

  function refresh() {
    void soundIpc.devices().then((d) => {
      devices = d;
      listed = true;
    });
  }
  refresh();

  function patch(p: Partial<Sound>) {
    void settings.patch('general', { sound: p });
  }

  /** The saved device, or null for "Same as Windows". */
  function picked(id: string): OutputDevice | null {
    return devices.find((d) => d.id === id) ?? null;
  }
</script>

{#snippet picker(label: string, current: OutputDevice | null, onpick: (d: OutputDevice | null) => void)}
  <select
    class="field"
    aria-label={label}
    value={current?.id ?? ''}
    onfocus={refresh}
    onchange={(e) => onpick(picked(e.currentTarget.value))}
  >
    <option value="">Same as Windows</option>
    {#each devices as d (d.id)}<option value={d.id}>{d.name}</option>{/each}
    {#if current && !devices.some((d) => d.id === current.id)}
      <option value={current.id}>{current.name}{listed ? ' (not connected)' : ''}</option>
    {/if}
  </select>
{/snippet}

<h2>Sound</h2>

<Row
  setting="general.sound.separateStreams"
  label="Streams on their own"
  hint="Windows’ volume mixer shows “cyex deck - streams” next to “cyex deck”. Off: one entry."
>
  <Toggle label="Streams on their own" checked={s.separateStreams} onchange={(v) => patch({ separateStreams: v })} />
</Row>

<Row
  setting="general.sound.streamsDevice"
  label="Streams play on"
  hint="Streams, VODs, clips and stories. Windows forgets its own choice for them when WebView2 updates; this one stays."
>
  {@render picker('Streams play on', s.streamsDevice, (d) => patch({ streamsDevice: d }))}
</Row>

<Row setting="general.sound.soundsDevice" label="Other sounds play on" hint="Mentions, highlights, live alerts and whispers.">
  <span class="pair">
    {@render picker('Other sounds play on', s.soundsDevice, (d) => patch({ soundsDevice: d }))}
    <IconButton icon="play" label="Play a test sound" onclick={() => void soundIpc.play(null)} />
  </span>
</Row>

<style>
  .field {
    width: 240px;
  }
  .pair {
    display: inline-flex;
    gap: 4px;
    align-items: center;
  }
</style>
