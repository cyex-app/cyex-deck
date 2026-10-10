<!-- 1.9 Picking the emote for the two channel-point emote rewards: a sub emote to
     unlock, or one of yours and how to modify it. -->
<script lang="ts">
  import { onMount } from 'svelte';

  import type { EmoteChoice } from '../../ipc/bindings/EmoteChoice';
  import type { EmoteRewardChoices } from '../../ipc/bindings/EmoteRewardChoices';
  import { chatIpc } from '../../ipc/chat';
  import { settings } from '../../lib/settings.svelte';
  import { emoteImg } from './urls';

  interface Props {
    view: number;
    kind: 'unlock' | 'modify';
    /** The emote (or modified emote) id to redeem, `null` until picked. */
    picked: string | null;
  }

  let { view, kind, picked = $bindable() }: Props = $props();

  let choices = $state<EmoteRewardChoices | null>(null);
  let error = $state('');
  let base = $state<EmoteChoice | null>(null);

  const theme = $derived(settings.v.appearance.theme);
  const list = $derived(choices ? (kind === 'unlock' ? choices.unlock : choices.modify) : []);
  const title = (modifier: string) => choices?.modifiers.find((m) => m.id === modifier)?.title ?? modifier;

  onMount(() => {
    chatIpc
      .emoteChoices(view)
      .then((c) => (choices = c))
      .catch((e) => (error = String(e)));
  });

  function img(id: string) {
    return emoteImg('twitch', id, theme);
  }
</script>

{#if error}
  <p class="note">{error}</p>
{:else if !choices}
  <p class="note">Loading…</p>
{:else if !list.length}
  <p class="note">{kind === 'unlock' ? 'You have every emote this one can unlock.' : 'Nothing left to modify.'}</p>
{:else if kind === 'modify' && base}
  <p class="note">How should {base.name} change?</p>
  <div class="grid">
    {#each base.modifications as m (m.id)}
      <button class="cell wide" class:on={picked === m.id} onclick={() => (picked = m.id)}>
        <img src={img(m.id).src} srcset={img(m.id).srcset} alt={m.name} />
        <span>{title(m.modifier)}</span>
      </button>
    {/each}
  </div>
  <button
    class="back"
    onclick={() => {
      base = null;
      picked = null;
    }}>Another emote</button
  >
{:else}
  <div class="grid">
    {#each list as e (e.id)}
      <button
        class="cell"
        class:on={picked === e.id}
        title={e.name}
        onclick={() => (kind === 'unlock' ? (picked = e.id) : (base = e))}
      >
        <img src={img(e.id).src} srcset={img(e.id).srcset} alt={e.name} />
      </button>
    {/each}
  </div>
{/if}

<style>
  .grid {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    max-height: 180px;
    overflow-y: auto;
  }
  .cell {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    padding: 0;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius);
  }
  .cell img {
    width: 28px;
    height: 28px;
  }
  .cell:hover {
    background: var(--surface-hover);
  }
  .cell.on {
    border-color: var(--accent-border);
    background: var(--accent-dim);
  }
  .wide {
    display: flex;
    gap: 8px;
    width: 100%;
    justify-content: flex-start;
    padding: 0 6px;
    font-size: 0.82rem;
  }
  .note {
    margin: 4px 0 8px;
    color: var(--dim);
    font-size: 0.8rem;
  }
  .back {
    margin-top: 6px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--accent-text);
    font-size: 0.8rem;
  }
</style>
