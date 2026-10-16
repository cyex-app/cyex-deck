<!-- The tickboxes before a sign-in (7TV, subscriptions and wallet), 7TV's consent when it's
     part of it, then each sign-in's progress in the same place. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Dialog from '../../components/Dialog.svelte';
  import Icon from '../../components/Icon.svelte';
  import { signIn } from '../accounts/signIn.svelte';
  import ConsentText from '../cosmetics/ConsentText.svelte';
  import Checkbox from '../../components/Checkbox.svelte';
  import { asksConsent, chosen, PART_NAME, type Part } from './choice';
  import { unlock, type Step } from './unlock.svelte';

  const asked = $derived(unlock.asked);
  const steps = $derived(unlock.steps);
  const items = $derived(asked?.items ?? []);
  let ticks = $state<Record<Part, boolean>>({ seventv: false, twitch: false });
  const parts = $derived<Part[]>(asked?.items ? chosen(items, ticks) : (asked?.parts ?? []));
  const consent = $derived(!!asked && asksConsent(parts, !asked.consent));

  // A new question starts on what its entry implies.
  $effect(() => {
    ticks = { seventv: false, twitch: false, ...Object.fromEntries((asked?.items ?? []).map((i) => [i.part, i.ticked])) };
  });

  const WHAT: Record<Part, string> = {
    seventv: 'Your 7TV paint and badge.',
    twitch: 'Your subs, payment methods, the miner and channel points.',
  };

  const title = $derived.by(() => {
    if (!asked) return '';
    if (!asked.items && consent) return `Use 7TV with ${asked.name}?`;
    if (asked.entry === 'seventv') return 'Sign in to 7TV';
    if (asked.entry === 'twitch') return 'Unlock subscriptions and wallet';
    return `Unlock for ${asked.name}`;
  });

  function stateText(s: Step): string {
    if (s.state === 'done') return 'Done';
    if (s.state === 'failed') return s.note || 'Didn’t finish';
    if (s.state === 'waiting') return 'Next';
    if (s.part === 'seventv') return 'Sign in in the 7TV window';
    return signIn.code ? `Approve ${signIn.code.userCode} in the Twitch window` : 'Opening Twitch…';
  }

  function go() {
    if (parts.length) void unlock.go(parts);
  }
</script>

{#if asked}
  <Dialog {title} width={440} top locked={!!steps} onclose={() => unlock.close()} onsubmit={steps ? undefined : go}>
    {#if steps}
      <ol class="steps" aria-live="polite">
        {#each steps as s (s.part)}
          <li class={s.state}>
            <span class="mark">
              {#if s.state === 'done'}<Icon name="check" size={14} />{:else if s.state === 'failed'}<Icon name="x" size={14} />{/if}
            </span>
            <strong>{PART_NAME[s.part]}</strong>
            <span class="state">{stateText(s)}</span>
          </li>
        {/each}
      </ol>
    {:else}
      {#if asked.items}
        <div class="ticks" role="group" aria-label="What to unlock">
          {#each items as it (it.part)}
            <div class="tick">
              <span data-autofocus={it === items.find((x) => !x.done) ? '' : undefined}>
                <Checkbox
                  checked={ticks[it.part]}
                  disabled={it.done}
                  label={it.label}
                  onchange={(v) => (ticks[it.part] = v)}
                />
              </span>
              {#if it.done}<span class="state">Done</span>{/if}
            </div>
          {/each}
        </div>
      {:else}
        <p class="what">{parts.map((p) => WHAT[p]).join(' ')}</p>
      {/if}
      {#if consent}<ConsentText name={asked.name} />{/if}
    {/if}
    {#snippet footer()}
      {#if steps}
        {#if signIn.kind}<Button variant="ghost" onclick={() => unlock.stop()}>Stop</Button>{/if}
      {:else}
        <Button variant="ghost" onclick={() => unlock.close()}>Cancel</Button>
        <Button variant="primary" disabled={!parts.length} onclick={go}>{consent ? 'Allow and sign in' : 'Sign in'}</Button>
      {/if}
    {/snippet}
  </Dialog>
{/if}

<style>
  .ticks {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 6px;
  }
  .tick {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .what {
    margin: 10px 0;
    color: var(--muted);
    line-height: 1.5;
  }
  .steps {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: grid;
    grid-template-columns: 16px auto 1fr;
    align-items: center;
    gap: 8px;
    min-height: 28px;
  }
  .mark {
    display: inline-flex;
    color: var(--muted);
  }
  li.done .mark {
    color: var(--green);
  }
  li.failed .mark,
  li.failed .state {
    color: var(--red);
  }
  li.waiting {
    color: var(--dim);
  }
  .state {
    justify-self: end;
    color: var(--muted);
    font-size: 0.85rem;
    text-align: right;
  }
  li.running .state {
    color: var(--text);
  }
</style>
