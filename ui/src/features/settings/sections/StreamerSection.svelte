<!-- U4 Streamer mode: when it turns on and what it keeps off your stream. -->
<script lang="ts">
  import SectionHead from '../SectionHead.svelte';
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { StreamerMode } from '../../../ipc/bindings/StreamerMode';
  import type { StreamerWhen } from '../../../ipc/bindings/StreamerWhen';
  import { settings } from '../../../lib/settings.svelte';
  import { streamer } from '../../../lib/streamer.svelte';

  const s = $derived(settings.v.general.streamer);

  const WHEN: { value: StreamerWhen; label: string }[] = [
    { value: 'auto', label: 'While streaming' },
    { value: 'on', label: 'Always' },
    { value: 'off', label: 'Off' },
  ];

  type Hide = 'hideLinkPreviews' | 'hideAvatars' | 'hideModActions' | 'hideWhispers' | 'noNotifications' | 'muteSounds';
  const HIDES: { key: Hide; label: string; hint: string }[] = [
    { key: 'hideLinkPreviews', label: 'Link previews', hint: 'Pictures and cards for links in chat.' },
    { key: 'hideAvatars', label: 'Profile pictures', hint: 'On usercards.' },
    { key: 'hideModActions', label: 'Mod actions', hint: 'Timeouts, bans and deleted messages.' },
    { key: 'hideWhispers', label: 'Whispers', hint: 'In chat, and their pop-ups and sounds.' },
    { key: 'noNotifications', label: 'Windows notifications', hint: 'Live alerts, whispers and the bell.' },
    { key: 'muteSounds', label: 'Sounds', hint: 'Mentions, highlights and whispers.' },
  ];

  const status = $derived(
    streamer.active
      ? streamer.state.forced
        ? 'On now, turned on by hand.'
        : streamer.state.appRunning
          ? 'On now: a streaming app is running.'
          : 'On now.'
      : streamer.state.forced
        ? 'Off for now, turned off by hand.'
        : 'Off now.',
  );

  const patch = (p: Partial<StreamerMode>) => settings.patch('general', { streamer: p });

  let adding = $state('');
  function addApp() {
    const name = adding.trim();
    if (!name) return;
    void patch({ apps: [...s.apps, name.toLowerCase().endsWith('.exe') ? name : `${name}.exe`] });
    adding = '';
  }
</script>

<SectionHead title="Streamer mode" level={2}>Keeps things off your stream that viewers shouldn’t see. {status}</SectionHead>

<Row setting="general.streamer.when" label="Turn on" hint="While streaming: when one of the apps below runs.">
  <Segmented label="Turn on" options={WHEN} value={s.when} onchange={(when) => patch({ when })} />
</Row>

{#if s.when === 'auto'}
  <Row setting="general.streamer.apps" label="Streaming apps" hint="Their program file names, like obs64.exe." stacked>
    <div class="apps">
      {#each s.apps as app (app)}
        <span class="chip"
          >{app}<button aria-label="Remove {app}" onclick={() => patch({ apps: s.apps.filter((a) => a !== app) })}
            >×</button
          ></span
        >
      {/each}
      <input
        class="field"
        placeholder="Add an app"
        aria-label="Add an app"
        bind:value={adding}
        onkeydown={(e) => e.key === 'Enter' && addApp()}
        onblur={addApp}
      />
    </div>
  </Row>
{/if}

<SectionHead title="Hide">What to keep off your screen while streamer mode is on.</SectionHead>
{#each HIDES as h (h.key)}
  <Row setting="general.streamer.{h.key}" label={h.label} hint={h.hint}>
    <Toggle label={h.label} checked={s[h.key]} onchange={(v) => patch({ [h.key]: v })} />
  </Row>
{/each}

<style>
  .apps {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 26px;
    padding: 0 4px 0 9px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    font: 0.78rem var(--font-mono);
  }
  .chip button {
    position: relative;
    padding: 0 4px;
    background: none;
    border: 0;
    color: var(--dim);
    cursor: pointer;
  }
  .chip button:hover {
    color: var(--text);
  }
  /* 24 px to hit: the glyph box is about 15 px. */
  .chip button::before {
    content: '';
    position: absolute;
    inset: -5px;
  }
  .field {
    width: 160px;
    height: 26px;
  }
</style>
