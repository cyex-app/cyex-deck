<!-- V1051-02/03 Pick your 7TV paint and badge. One panel for the view and the in-chat
     pop-up (`compact`). Nothing talks to 7TV until the account has said yes. -->
<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import Button from '../../components/Button.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import { systemIpc } from '../../ipc/system';
  import { accounts } from '../../lib/accounts.svelte';
  import { keyTaken } from '../../lib/escape';
  import { isFindKey, visibleSearchbox } from '../../lib/findKey';
  import { focus } from '../../lib/focus.svelte';
    import { ComboStore } from './combos.svelte';
  import ComboStrip from './ComboStrip.svelte';
  import { CosmeticsStore } from './cosmetics.svelte';
  import { countLabel } from './filter';
  import Consent from './Consent.svelte';
  import Pane from './Pane.svelte';
  import Preview from './Preview.svelte';
  import Status from './Status.svelte';
  import StateCard from './StateCard.svelte';

  interface Props {
    account: string;
    compact: boolean;
    /** Cancel on the consent, and Escape in the pop-up. */
    onclose?: () => void;
  }

  let { account, compact, onclose }: Props = $props();

  // The view remounts the panel per account, so the store never outlives its account.
  // svelte-ignore state_referenced_locally
  const store = new CosmeticsStore(account);
  // svelte-ignore state_referenced_locally
  const combos = new ComboStore(store);
  const who = $derived(accounts.list.find((a) => a.userId === account));
  const view = $derived(store.view);
  const status = $derived(store.status);
  let root = $state<HTMLElement>();
  let tab = $state<'paints' | 'badges'>('paints');

  onMount(() => void store.load());
  onDestroy(() => store.destroy());

  // A sign-in or sign-out elsewhere (Settings) changes what the menu may do.
  $effect(() => {
    const live = store.liveKey();
    if (!store.loading && store.key && live !== store.key) void store.load();
  });

  const items = $derived(view && (status === 'ready' || status === 'signedOut' || status === 'expired'));

  /** A pick goes to 7TV, or into the draft while a combo is edited. */
  function pick(kind: 'paint' | 'badge', id: string | null, usable: boolean) {
    if (combos.editing && usable) combos.pick(kind, id);
    else store.pick(kind, id, usable);
  }

  /** Ctrl+F in the full view, like Settings: the search takes focus wherever focus is. */
  function onfind(e: KeyboardEvent) {
    if (compact || !isFindKey(e) || e.isComposing || keyTaken(e)) return;
    const box = visibleSearchbox(root ?? document);
    if (!box) return;
    e.preventDefault();
    box.focus();
    box.select();
  }

  function onkeydown(e: KeyboardEvent) {
    const typing = e.target instanceof HTMLInputElement;
    if ((e.key === '/' && !typing) || (e.ctrlKey && e.key.toLowerCase() === 'f')) {
      e.preventDefault();
      root?.querySelector<HTMLInputElement>('input[role="searchbox"]')?.focus();
    } else if (e.ctrlKey && !typing && e.key.toLowerCase() === 'z') {
      e.preventDefault();
      store.undo();
    } else if (e.key === 'Escape') {
      if (combos.editing) {
        combos.cancel();
        e.stopPropagation();
      } else if (compact) onclose?.();
    }
  }
</script>

<svelte:window onkeydown={onfind} />

{#snippet editBar()}
  <span>Editing “{combos.editing?.name}”. Pick a paint and a badge.</span>
  <Button variant="primary" small onclick={() => void combos.commit()}>Save</Button>
  <Button variant="ghost" small onclick={() => combos.cancel()}>Cancel</Button>
{/snippet}

<!-- Everything inside stands still while the window isn't in front: grid, combos, preview. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="panel" class:compact class:still={focus.idle} bind:this={root} {onkeydown}>
  {#if !who}
    <!-- The account left; nothing to show. -->
  {:else if status === 'noConsent' && view}
    <Consent name={who.displayName} inline={!compact} onallow={() => void store.allow()} oncancel={() => onclose?.()} />
  {:else if status === 'noAccount'}
    <StateCard kind="noAccount" onretry={() => void store.load(true)} />
  {:else if status === 'down'}
    <StateCard kind="down" onretry={() => void store.load(true)} />
  {:else if items && view}
    {#if (status === 'signedOut' && store.anyUsable) || status === 'expired' || !view.subscribed}
      <div class="bar">
        {#if status === 'signedOut' && store.anyUsable}
          <span>Sign in to 7TV to change your paint and badge.</span>
          <Button variant="primary" small onclick={() => store.askSignIn(false)}>Sign in to 7TV</Button>
        {:else if status === 'expired'}
          <span>Your 7TV sign-in ran out.</span>
          <Button variant="primary" small onclick={() => store.askSignIn(true)}>Sign in again</Button>
        {/if}
        {#if !view.subscribed}
          <span class="note">
            Paints and badges are for 7TV subscribers.{store.anyUsable ? ' Some don’t need it.' : ''}
            <Button variant="link" onclick={() => void systemIpc.openLink('https://7tv.app/store')}>7TV store</Button>
          </span>
        {/if}
      </div>
    {/if}

    <ComboStrip {combos} {store} name={who.displayName} />

    <!-- In the chat pop-up you are already in chat: no preview. -->
    {#if !compact}
      <Preview {store} {who} shown={combos.shown} />
    {/if}

    <!-- Pop-up: the switch and the status line share one row, so no blank strip and no jump. -->
    <div class="tabs" class:compact>
      {#if compact}
        <Segmented
          neutral
          label="Picker"
          value={tab}
          options={[
            { value: 'paints', label: `Paints · ${countLabel(store.paintsShown.length, view.paints.length)}` },
            { value: 'badges', label: `Badges · ${countLabel(store.badgesShown.length, view.badges.length)}` },
          ]}
          onchange={(t) => (tab = t)}
        />
      {/if}
      <!-- Next to the grid you pick from, not up by the preview. -->
      <Status {store} bar={combos.editing ? editBar : undefined} />
    </div>
    <div class="picker" class:compact>
      {#if !compact || tab === 'paints'}
        <Pane
          kind="paint"
          heading={!compact}
          {compact}
          query={store.paintsQ}
          onquery={(q) => (store.paintsQ = q)}
          rows={store.paintsShown}
          total={view.paints.length}
          selected={combos.shown.paint}
          name={who.displayName}
          onpick={(id, usable) => pick('paint', id, usable)}
        />
      {/if}
      {#if !compact || tab === 'badges'}
        <Pane
          kind="badge"
          heading={!compact}
          {compact}
          query={store.badgesQ}
          onquery={(q) => (store.badgesQ = q)}
          rows={store.badgesShown}
          total={view.badges.length}
          selected={combos.shown.badge}
          name={who.displayName}
          onpick={(id, usable) => pick('badge', id, usable)}
        />
      {/if}
    </div>
  {/if}
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100%;
    min-height: 0;
    container-type: inline-size;
  }
  /* The pop-up is small: tighter, and it scrolls as a last resort before the grid gets tiny. */
  .panel.compact {
    gap: 8px;
    overflow-y: auto;
  }
  .picker.compact {
    min-height: 120px;
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .tabs > :global(.status) {
    flex: 1;
    min-width: 0;
    justify-content: flex-end;
  }
  .tabs.compact > :global(.status) {
    min-height: 0;
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: 0.85rem;
  }
  .note {
    color: var(--dim);
  }
  .picker {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr);
    flex: 1;
    gap: 16px;
    min-height: 0;
  }
  @container (min-width: 720px) {
    .picker:not(.compact) {
      /* Widths follow the cells: 8 paints (8x176 + 7x6) and 6 badges (6x72 + 5x6), each plus ~16 px
         for the scrollbar. Narrower windows shrink both in that ratio. */
      grid-template-columns: minmax(0, 1466fr) minmax(0, 478fr);
    }
  }
  @container (max-width: 719px) {
    .picker:not(.compact) {
      /* Only the grids scroll; the page does only when the window is very short. */
      grid-template-rows: repeat(2, minmax(150px, 1fr));
      overflow-y: auto;
    }
  }
</style>
