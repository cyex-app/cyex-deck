<!-- V111-06 What Chatterino's commands bring: each command as it comes in (notes, names in use,
     what can't come and why) and the message-menu choice. V111-60 Shared by the commands dialog
     and Settings › Import. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Checkbox from '../../../components/Checkbox.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import type { CommandItem } from '../../../ipc/bindings/CommandItem';
  import type { CommandPreview } from '../../../ipc/bindings/CommandPreview';
  import { formatNumber } from '../../../lib/format';
  import { headline, isConflict, limitText, outcome, pickText } from './commandImport';
  import SkippedList from './SkippedList.svelte';

  const MINE = [
    { value: 'keep', label: 'Keep mine' },
    { value: 'take', label: 'Use Chatterino’s' },
  ] as const;
  const BUILT_IN = [
    { value: 'keep', label: 'Keep cyex’s' },
    { value: 'take', label: 'Use Chatterino’s' },
  ] as const;

  interface Props {
    preview: CommandPreview;
    take: Set<number>;
    menu: boolean;
  }

  let { preview, take = $bindable(), menu = $bindable() }: Props = $props();

  const result = $derived(outcome(preview, take));
  const conflicts = $derived(preview.items.filter(isConflict));
  const limit = $derived(limitText(preview, result));

  function choose(i: CommandItem, v: 'keep' | 'take') {
    const next = new Set(take);
    if (v === 'take') next.add(i.id);
    else next.delete(i.id);
    take = next;
  }

  function all(v: 'keep' | 'take') {
    take = v === 'take' ? new Set(conflicts.map((i) => i.id)) : new Set();
  }
</script>

<p class="head">{headline(preview)}</p>
{#if conflicts.length > 1}
  <div class="all">
    <span class="why">Names in use:</span>
    <Button small variant="ghost" onclick={() => all('keep')}>Keep all current</Button>
    <Button small variant="ghost" onclick={() => all('take')}>Use all Chatterino’s</Button>
  </div>
{/if}
{#if preview.items.length}
  <ul class="items">
    {#each preview.items as i (i.id)}
      <li>
        <code>{i.row}</code>
        <span class="does">{i.does}</span>
        {#each i.notes as n, k (k)}<span class="why">{n}</span>{/each}
        {#if i.conflict === 'yours'}
          <span class="why">You have <code>{i.yours}</code></span>
        {:else if i.conflict === 'builtIn'}
          <span class="why">cyex has its own command with this name.</span>
        {/if}
        {#if isConflict(i)}
          <Segmented
            options={i.conflict === 'yours' ? MINE : BUILT_IN}
            value={take.has(i.id) ? 'take' : 'keep'}
            label={`${i.name}: name in use`}
            onchange={(v) => choose(i, v)}
          />
          <!-- V111-55 Say what the pick does: the count in the button was the only change. -->
          <span class="why">{pickText(i, take.has(i.id))}</span>
        {/if}
        {#if i.menuNote}<span class="why">Not for the message menu: {i.menuNote}</span>{/if}
      </li>
    {/each}
  </ul>
{/if}
{#if result.menu}
  <Checkbox
    checked={menu}
    label={`Add ${formatNumber(result.menu)} to the message menu too`}
    hint="Chatterino showed these on right-click. They go under Commands."
    onchange={(v) => (menu = v)}
  />
{/if}
{#if limit}<p class="warn">{limit}</p>{/if}
{#if preview.unread}<p class="why">Only the first 2.000 were read; {formatNumber(preview.unread)} more stay out.</p>{/if}
<SkippedList list={preview.skipped} />

<style>
  .head {
    margin: 0 0 8px;
    font-weight: 600;
  }
  .does {
    display: block;
    margin: 2px 0;
    font-size: var(--text-hint);
  }
  .why {
    display: block;
    margin: 2px 0;
    font-size: var(--text-hint);
    color: var(--dim);
  }
  p.why {
    margin: 6px 0;
  }
  .warn {
    margin: 6px 0;
    font-size: var(--text-hint);
    color: var(--red);
  }
  .all {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 8px;
  }
  .all .why {
    display: inline;
  }
  ul {
    margin: 6px 0 8px;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 6px;
    font-size: var(--text-sub);
  }
  .items {
    max-height: 320px;
    overflow-y: auto;
  }
  .items li {
    display: grid;
    gap: 2px;
    justify-items: start;
  }
  li {
    overflow-wrap: anywhere;
  }
  code {
    font-family: var(--font-mono);
    font-size: 0.78rem;
  }
</style>
