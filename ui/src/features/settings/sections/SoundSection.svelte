<!-- V1051-01 Settings → Sound: the app in Windows' volume mixer, and where streams and the
     other sounds play. Changes apply at once. -->
<script lang="ts">
  import Select from '../../../components/Select.svelte';
  import IconButton from '../../../components/IconButton.svelte';
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { OutputDevice } from '../../../ipc/bindings/OutputDevice';
  import type { Sound } from '../../../ipc/bindings/Sound';
  import { soundIpc } from '../../../ipc/sound';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';

  const s = $derived(settings.v.general.sound);
  let devices = $state<OutputDevice[]>([]);
  let defaultName = $state('');
  /** Until the first list arrives, a saved device isn't called "not connected". */
  let listed = $state(false);

  function refresh() {
    soundIpc
      .devices()
      .then(({ devices: d, defaultName: def }) => {
        defaultName = def;
        devices = [...d].sort((a, b) => a.name.localeCompare(b.name));
        listed = true;
      })
      .catch(() => {
        // Keep the last list; the saved device isn't called "not connected" on a guess.
      });
  }
  refresh();

  function patch(p: Partial<Sound>) {
    void settings.patch('general', { sound: p });
  }

  /** The picked device (also the saved one that isn’t plugged in), or null for "Same as Windows". */
  function picked(id: string, current: OutputDevice | null): OutputDevice | null {
    return devices.find((d) => d.id === id) ?? (current?.id === id ? current : null);
  }
  /** "Same as Windows · Speakers", the plugged-in devices, and the saved one that isn’t. */
  function choices(current: OutputDevice | null) {
    const list = [{ value: '', label: defaultName ? `Same as Windows · ${defaultName}` : 'Same as Windows' }];
    for (const d of devices) list.push({ value: d.id, label: d.name });
    if (current && !devices.some((d) => d.id === current.id)) {
      list.push({ value: current.id, label: `${current.name}${listed ? ' (not connected)' : ''}` });
    }
    return list;
  }
</script>

{#snippet picker(label: string, current: OutputDevice | null, onpick: (d: OutputDevice | null) => void)}
  <span class="sel">
    <Select
      {label}
      width="100%"
      value={current?.id ?? ''}
      options={choices(current)}
      onopen={refresh}
      onchange={(id) => onpick(picked(id, current))}
    />
  </span>
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
  hint="Streams, VODs, clips and stories. Pick it here so it sticks after updates."
  stacked
>
  <span class="pair">
    {@render picker('Streams play on', s.streamsDevice, (d) => patch({ streamsDevice: d }))}
    <!-- Where the test button sits in the next row, so both lists start at the same x. -->
    <span class="gap" aria-hidden="true"></span>
  </span>
</Row>

<Row setting="general.sound.soundsDevice" label="Other sounds play on" hint="Mentions, highlights, live alerts and whispers." stacked>
  <span class="pair">
    {@render picker('Other sounds play on', s.soundsDevice, (d) => patch({ soundsDevice: d }))}
    <IconButton icon="play" label="Play a test sound" onclick={() => void soundIpc.play(null).catch(() => toast.error('Couldn’t play the sound.'))} />
  </span>
</Row>

<Row
  setting="general.sound.playInFront"
  label="Play sounds while cyex deck is in front"
  hint="Highlights, mentions, whispers and live alerts. Off: they only play while you’re in another app."
>
  <Toggle label="Play sounds while cyex deck is in front" checked={s.playInFront} onchange={(v) => patch({ playInFront: v })} />
</Row>

<style>
  .gap {
    flex: none;
    width: 30px;
  }
  /* Both lists end at the column's edge, whether or not the test button is there. */
  .pair {
    display: flex;
    width: 100%;
    gap: 4px;
    align-items: center;
  }
  .sel {
    flex: 1 1 0;
    min-width: 0;
  }
</style>
