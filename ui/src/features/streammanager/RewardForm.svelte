<!-- V107-R37 A channel points reward's fields, for a new one or an edit. Rust checks them
     (`sm_reward_check`) and saves; the first problem shows under the fields. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Checkbox from '../../components/Checkbox.svelte';
  import NumberField from '../../components/NumberField.svelte';
  import type { SmRewardEdit } from '../../ipc/bindings/SmRewardEdit';
  import { rewardsIpc } from '../../ipc/rewards';
  import { formatSecs, parseDuration } from '../../lib/format';
  import { COLORS, sameEdit } from './rewardsLogic';

  interface Props {
    start: SmRewardEdit;
    /** An edit, not a new reward. */
    editing: boolean;
    busy: boolean;
    onsave: (edit: SmRewardEdit) => void;
    oncancel: () => void;
  }

  let { start, editing, busy, onsave, oncancel }: Props = $props();

  // A copy to edit; `start` stays as it was.
  // svelte-ignore state_referenced_locally
  let e = $state<SmRewardEdit>({ ...start });
  // svelte-ignore state_referenced_locally
  let cooldownText = $state(start.cooldown ? formatSecs(start.cooldown) : '5m');
  let problems = $state<string[]>([]);

  const cooldownSecs = $derived(parseDuration(cooldownText));
  const cooldownOk = $derived(e.cooldown === null || cooldownSecs !== null);
  const changed = $derived(!editing || !sameEdit(e, start));

  $effect(() => {
    const snapshot = { ...e };
    const t = setTimeout(() => {
      rewardsIpc
        .check(snapshot)
        .then((p) => (problems = p))
        .catch(() => {});
    }, 150);
    return () => clearTimeout(t);
  });

  function setCooldown(text: string) {
    cooldownText = text;
    const s = parseDuration(text);
    if (e.cooldown !== null && s !== null) e.cooldown = s;
  }

  function limit(key: 'maxPerStream' | 'maxPerUser', on: boolean, n: number) {
    e[key] = on ? n : null;
  }
</script>

<form
  class="form"
  onsubmit={(ev) => {
    ev.preventDefault();
    if (!problems.length && cooldownOk && changed) onsave({ ...e, title: e.title.trim(), prompt: e.prompt.trim() });
  }}
>
  <div class="line">
    <input class="field grow" placeholder="Name" maxlength="45" bind:value={e.title} aria-label="Name" />
    <NumberField label="Cost in points" min={1} max={1000000000} width="110px" unit="pts" value={e.cost} onchange={(n) => (e.cost = n)} />
  </div>
  <textarea class="field area" placeholder="Description (optional)" maxlength="200" rows="2" bind:value={e.prompt} aria-label="Description"></textarea>
  <div class="colors" role="radiogroup" aria-label="Color">
    {#each COLORS as c (c)}
      <button
        type="button"
        class="swatch"
        class:on={e.color === c}
        style:background={c}
        role="radio"
        aria-checked={e.color === c}
        aria-label="Color {c}"
        onclick={() => (e.color = c)}
      ></button>
    {/each}
    <label class="pick" aria-label="Other color">
      <input type="color" value={e.color.toLowerCase()} oninput={(ev) => (e.color = ev.currentTarget.value.toUpperCase())} />
    </label>
  </div>
  <div class="opts">
    <Checkbox label="Viewer types a message" checked={e.inputRequired} onchange={(on) => (e.inputRequired = on)} />
    <Checkbox
      label="Skip the queue"
      hint="Redemptions count as done right away and never wait for you."
      checked={e.skipQueue}
      onchange={(on) => (e.skipQueue = on)}
    />
    <div class="line">
      <Checkbox label="Cooldown" checked={e.cooldown !== null} onchange={(on) => (e.cooldown = on ? (cooldownSecs ?? 300) : null)} />
      {#if e.cooldown !== null}
        <input
          class="field short"
          class:bad={!cooldownOk}
          placeholder="5m"
          aria-label="Cooldown, e.g. 30s, 5m or 1h"
          value={cooldownText}
          oninput={(ev) => setCooldown(ev.currentTarget.value)}
        />
      {/if}
    </div>
    <div class="line">
      <Checkbox label="Limit per stream" checked={e.maxPerStream !== null} onchange={(on) => limit('maxPerStream', on, 10)} />
      {#if e.maxPerStream !== null}
        <NumberField label="Per stream" min={1} max={1000000} width="90px" value={e.maxPerStream} onchange={(n) => (e.maxPerStream = n)} />
      {/if}
    </div>
    <div class="line">
      <Checkbox label="Limit per viewer" checked={e.maxPerUser !== null} onchange={(on) => limit('maxPerUser', on, 1)} />
      {#if e.maxPerUser !== null}
        <NumberField label="Per viewer per stream" min={1} max={1000000} width="90px" value={e.maxPerUser} onchange={(n) => (e.maxPerUser = n)} />
      {/if}
    </div>
  </div>
  {#if !cooldownOk}
    <p class="note bad-text">Cooldowns look like 30s, 5m or 1h.</p>
  {:else if problems.length && e.title.trim()}
    <p class="note bad-text">{problems[0]}</p>
  {/if}
  <div class="line">
    <Button small variant="primary" type="submit" disabled={busy || !cooldownOk || problems.length > 0 || !changed}>
      {editing ? 'Save' : 'Create'}
    </Button>
    <Button small variant="ghost" onclick={oncancel}>Cancel</Button>
  </div>
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .form :global(.field) {
    background: var(--surface);
  }
  .grow {
    flex: 1 1 140px;
  }
  .area {
    height: auto;
    padding: 6px 10px;
    resize: vertical;
    font: inherit;
    font-size: 0.85rem;
  }
  .short {
    width: 80px;
  }
  .bad {
    border-color: var(--red);
  }
  .bad-text {
    color: var(--red) !important;
  }
  .colors {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .swatch,
  .pick {
    width: 20px;
    height: 20px;
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    cursor: pointer;
  }
  .swatch.on {
    outline: 2px solid var(--text);
    outline-offset: 1px;
  }
  .pick {
    position: relative;
    overflow: hidden;
    background: conic-gradient(#fa2828, #ffca5f, #00c7ac, #145aff, #9147ff, #fa2828);
  }
  .pick input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }
  .opts {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .opts .line {
    min-height: 28px;
  }
</style>
