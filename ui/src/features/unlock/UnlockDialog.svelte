<!-- The tickboxes before a sign-in (7TV, subscriptions and wallet), 7TV's consent when it's
     part of it, then the sign-in itself in the same place. V108-7U While it runs: the account,
     what's happening now (a spinner, a tick or a cross with a short line), and the actions
     that fit: Cancel, 7TV from the browser instead, Try again. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import Checkbox from '../../components/Checkbox.svelte';
  import Dialog from '../../components/Dialog.svelte';
  import Icon from '../../components/Icon.svelte';
  import { accounts } from '../../lib/accounts.svelte';
  import { signIn } from '../accounts/signIn.svelte';
  import SevenTvPaste from '../accounts/SevenTvPaste.svelte';
  import ConsentText from '../cosmetics/ConsentText.svelte';
  import { asksConsent, chosen, PART_NAME, type Part } from './choice';
  import { doneLead, statusOf } from './status';
  import { unlock } from './unlock.svelte';

  const asked = $derived(unlock.asked);
  const steps = $derived(unlock.steps);
  const items = $derived(asked?.items ?? []);
  let ticks = $state<Record<Part, boolean>>({ seventv: false, twitch: false });
  const parts = $derived<Part[]>(asked?.items ? chosen(items, ticks) : (asked?.parts ?? []));
  const consent = $derived(!!asked && asksConsent(parts, !asked.consent));
  const account = $derived(asked ? accounts.list.find((a) => a.userId === asked.account) : undefined);

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
    if (!asked.items && consent && !steps) return `Use 7TV with ${asked.name}?`;
    if (asked.entry === 'seventv' || (steps?.length === 1 && steps[0].part === 'seventv')) return 'Sign in to 7TV';
    if (asked.entry === 'twitch') return 'Unlock subscriptions and wallet';
    return `Unlock for ${asked.name}`;
  });

  const current = $derived(unlock.current);
  const allDone = $derived(!!steps && steps.every((s) => s.state === 'done'));
  const status = $derived.by(() => {
    if (!steps) return null;
    if (allDone) return { lead: doneLead(steps.map((s) => s.part)), detail: '' };
    if (!current) return null;
    return statusOf(current.part, current.state, {
      stage: current.stage,
      note: current.note,
      code: signIn.code?.userCode ?? null,
      opened: signIn.opened,
    });
  });
  /** The glyph beside the line: busy, done or failed. */
  const look = $derived(allDone ? 'done' : current?.state === 'failed' ? 'failed' : 'busy');
  const failedParts = $derived(steps?.filter((s) => s.state === 'failed') ?? []);
  const sevenTvLeft = $derived(failedParts.some((s) => s.part === 'seventv') || (unlock.running && current?.part === 'seventv' && current.stage !== 'connecting'));

  function go() {
    if (parts.length) void unlock.go(parts);
  }

  function stepText(state: string): string {
    if (state === 'done') return 'Done';
    if (state === 'failed') return 'Didn’t finish';
    if (state === 'running') return 'Now';
    return 'Next';
  }
</script>

{#snippet askFooter()}
  <Button variant="ghost" onclick={() => unlock.close()}>Cancel</Button>
  <Button variant="primary" disabled={!parts.length} onclick={go}>{consent ? 'Allow and sign in' : 'Sign in'}</Button>
{/snippet}

{#snippet runFooter()}
  {#if unlock.browser}
    <span class="lead-action"><Button variant="ghost" onclick={() => void unlock.retry()}>Use the 7TV window</Button></span>
    <Button variant="ghost" onclick={() => unlock.close()}>Cancel</Button>
  {:else if unlock.running}
    {#if sevenTvLeft}<span class="lead-action"><Button variant="ghost" onclick={() => unlock.useBrowser()}>Use your browser instead</Button></span>{/if}
    <Button variant="ghost" onclick={() => unlock.cancel()}>Cancel</Button>
  {:else}
    {#if sevenTvLeft}<span class="lead-action"><Button variant="ghost" onclick={() => unlock.useBrowser()}>Use your browser instead</Button></span>{/if}
    <Button variant="ghost" onclick={() => unlock.close()}>Close</Button>
    <Button variant="primary" onclick={() => void unlock.retry()}>Try again</Button>
  {/if}
{/snippet}

{#if asked}
  <Dialog
    {title}
    width={440}
    top
    locked={unlock.running}
    onclose={() => unlock.close()}
    onsubmit={steps ? undefined : go}
    footer={!steps ? askFooter : allDone ? undefined : runFooter}
  >
    {#if steps}
      <div class="run">
        <div class="who">
          <Avatar src={account?.avatar} name={asked.name} size={28} />
          <span class="name">{asked.name}</span>
        </div>
        {#if unlock.browser && account}
          <SevenTvPaste {account} bare onclose={() => unlock.close()} onconnected={() => unlock.browserDone()} />
        {:else if status}
          {#key `${look}:${status.lead}`}
            <div class="status {look}" role="status" aria-live="polite">
              <span class="glyph" aria-hidden="true">
                {#if look === 'done'}<Icon name="check" size={16} />{:else if look === 'failed'}<Icon name="x" size={16} />{:else}<span class="spinner"></span>{/if}
              </span>
              <span class="text">
                <span class="lead">{status.lead}</span>
                {#if status.detail}<span class="detail">{status.detail}</span>{/if}
              </span>
            </div>
          {/key}
        {/if}
        {#if steps.length > 1 && !unlock.browser}
          <ol class="steps" aria-label="Steps">
            {#each steps as s (s.part)}
              <li class={s.state}>
                <span>{PART_NAME[s.part]}</span>
                <span class="state">{stepText(s.state)}</span>
              </li>
            {/each}
          </ol>
        {/if}
      </div>
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
  .run {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-top: 4px;
  }
  .who {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .name {
    overflow: hidden;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr);
    align-items: start;
    gap: 12px;
    padding: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    animation: appear var(--fast) ease-out;
  }
  .glyph {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    color: var(--muted);
  }
  .done .glyph {
    color: var(--green);
  }
  .failed .glyph {
    color: var(--red);
  }
  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--border-strong);
    border-top-color: var(--muted);
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    padding-top: 1px;
    overflow-wrap: anywhere;
  }
  .lead {
    font-weight: 600;
    line-height: 1.4;
  }
  .detail {
    color: var(--muted);
    font-size: var(--text-sub);
    line-height: 1.45;
  }
  .steps {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 0.85rem;
  }
  .steps li {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    color: var(--muted);
  }
  .steps li.running {
    color: var(--text);
  }
  .steps li.done .state {
    color: var(--green);
  }
  .steps li.failed .state {
    color: var(--red);
  }
  .steps li.waiting {
    color: var(--dim);
  }
  .state {
    color: var(--muted);
    font-size: 0.85rem;
  }
  /* The odd one out sits at the start of the footer, away from Cancel and the main action. */
  .lead-action {
    margin-right: auto;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes appear {
    from {
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spinner,
    .status {
      animation: none;
    }
  }
</style>
