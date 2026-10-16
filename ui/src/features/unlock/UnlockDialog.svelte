<!-- The choice before a sign-in (7TV, subscriptions and wallet, or both), 7TV's consent when it's
     part of it, then each sign-in's progress in the same place. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Dialog from '../../components/Dialog.svelte';
  import Icon from '../../components/Icon.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import { signIn } from '../accounts/signIn.svelte';
  import ConsentText from '../cosmetics/ConsentText.svelte';
  import { asksConsent, PART_NAME, type Part } from './choice';
  import { unlock, type Step } from './unlock.svelte';

  const asked = $derived(unlock.asked);
  const steps = $derived(unlock.steps);
  const options = $derived(asked?.options?.filter((o) => !o.done) ?? []);
  let picked = $state(0);
  const parts = $derived<Part[]>(asked?.options ? (options[picked]?.parts ?? []) : (asked?.parts ?? []));
  const consent = $derived(!!asked && asksConsent(parts, !asked.consent));

  // A new question starts on its first choice.
  $effect(() => {
    void asked;
    picked = 0;
  });

  const WHAT: Record<Part, string> = {
    seventv: 'Your 7TV paint and badge.',
    twitch: 'Your subs, payment methods, the miner and channel points.',
  };

  const title = $derived.by(() => {
    if (!asked) return '';
    if (!asked.options && consent) return `Use 7TV with ${asked.name}?`;
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
  <Dialog {title} width={440} locked={!!steps} onclose={() => unlock.close()} onsubmit={steps ? undefined : go}>
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
      {#if asked.options}
        <Segmented
          label="What to unlock"
          options={options.map((o, i) => ({ value: i, label: o.label }))}
          value={picked}
          onchange={(i) => (picked = i)}
        />
      {/if}
      <p class="what">{parts.map((p) => WHAT[p]).join(' ')}{parts.length > 1 ? ' Twitch first, then 7TV.' : ''}</p>
      {#if consent}<ConsentText name={asked.name} />{/if}
    {/if}
    {#snippet footer()}
      {#if steps}
        {#if signIn.kind}<Button variant="ghost" onclick={() => unlock.stop()}>Stop</Button>{/if}
      {:else}
        <Button variant="ghost" onclick={() => unlock.close()}>Cancel</Button>
        <Button variant="primary" onclick={go}>{consent ? 'Allow and sign in' : 'Sign in'}</Button>
      {/if}
    {/snippet}
  </Dialog>
{/if}

<style>
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
