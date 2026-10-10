<!-- 1.35 The stream manager: your own channel at a glance and the tools to run it. A fixed
     view in the main window that pops out (route `streammanager`), e.g. onto a second
     screen. Needs a Twitch sign-in for your channel (D26). -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { uptimeShort } from '../browse/lists';
  import ActivityCard from './ActivityCard.svelte';
  import AdsCard from './AdsCard.svelte';
  import InfoCard from './InfoCard.svelte';
  import MarkersCard from './MarkersCard.svelte';
  import ModesCard from './ModesCard.svelte';
  import PollCard from './PollCard.svelte';
  import PredictionCard from './PredictionCard.svelte';
  import RaidCard from './RaidCard.svelte';
  import StreamCard from './StreamCard.svelte';
  import { sm } from './streammanager.svelte';
  import TogetherCard from './TogetherCard.svelte';

  interface Props {
    /** Shown in its own window. */
    popout?: boolean;
    route?: string;
    onpopout?: () => void;
    onsignin?: () => void;
  }

  let { popout = false, onpopout, onsignin }: Props = $props();

  $effect(() => {
    void sm.open();
    return () => sm.close();
  });

  let now = $state(Date.now());
  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(t);
  });

  const m = $derived(sm.manager);
  const s = $derived(sm.stats);
</script>

<div class="sm">
  {#if !sm.ready}
    <!-- Reading what Rust has. -->
  {:else if !m}
    <div class="empty">
      <EmptyState icon="radio" text="Sign in with Twitch for your own channel to run your stream from here." />
      {#if onsignin}<Button small onclick={onsignin}>Sign in</Button>{/if}
    </div>
  {:else}
    <header>
      <Avatar src={m.avatar} name={m.display} size={28} />
      <h2>{m.display}</h2>
      {#if s?.live}
        <span class="live">Live</span>
        <span class="up">{uptimeShort(s.startedAt, now)}</span>
      {:else if s}
        <span class="off">Offline</span>
      {/if}
      <span class="spacer"></span>
      {#if !popout && onpopout}
        <IconButton icon="popout" label="Pop out" hint="Open the stream manager in its own window." size={15} onclick={onpopout} />
      {/if}
    </header>
    <div class="grid">
      <StreamCard />
      <InfoCard />
      <ActivityCard />
      <PollCard login={m.login} />
      <PredictionCard login={m.login} />
      <ModesCard />
      <AdsCard />
      <RaidCard />
      <MarkersCard />
      <TogetherCard />
    </div>
  {/if}
</div>

<style>
  .sm {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    height: 100%;
    background: var(--bg);
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  h2 {
    margin: 0;
    font-size: 0.95rem;
  }
  .live {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--red);
    font-size: 0.75rem;
    font-weight: 700;
  }
  .live::before {
    content: '';
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--red);
  }
  .up,
  .off {
    color: var(--muted);
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }
  .spacer {
    flex: 1;
  }
  .grid {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 10px;
    padding: 10px;
    align-content: start;
  }
</style>
