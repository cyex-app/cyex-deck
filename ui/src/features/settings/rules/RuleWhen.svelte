<!-- V110-C59 A rule's conditions, under its row: a chip with the summary while set and
     closed, the editor while open. Every field set must hold; nothing set means always. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Checkbox from '../../../components/Checkbox.svelte';
  import NumberField from '../../../components/NumberField.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import type { LiveWhen } from '../../../ipc/bindings/LiveWhen';
  import type { RoleWhen } from '../../../ipc/bindings/RoleWhen';
  import type { When } from '../../../ipc/bindings/When';
  import { cleanWhen, NO_WHEN, splitCategories, splitChannels, whenSummary } from './when';

  interface Props {
    when: When | null | undefined;
    open: boolean;
    onopen: () => void;
    onchange: (when: When | null) => void;
  }

  let { when, open, onopen, onchange }: Props = $props();

  const w = $derived(when ?? NO_WHEN);
  const summary = $derived(whenSummary(when));

  function set(patch: Partial<When>) {
    onchange(cleanWhen({ ...w, ...patch }));
  }

  /** V110-F24 A typed list is written back cleaned, so the box shows what was kept even when the
   * list is the same as before (and the box's own value wouldn't change). */
  function typed(e: Event & { currentTarget: HTMLInputElement }, split: (t: string) => string[], key: 'channels' | 'categories') {
    const list = split(e.currentTarget.value);
    e.currentTarget.value = list.join(', ');
    set({ [key]: list });
  }

  const SCOPE = [
    { value: 'only', label: 'Only there' },
    { value: 'never', label: 'Never there' },
  ] as const;
  const LIVE = [
    { value: 'any', label: 'Any' },
    { value: 'live', label: 'Live' },
    { value: 'offline', label: 'Offline' },
  ] as const;
  const ROLE = [
    { value: 'any', label: 'Any' },
    { value: 'vip', label: 'VIP' },
    { value: 'mod', label: 'Mod' },
    { value: 'broadcaster', label: 'Broadcaster' },
  ] as const;
</script>

{#if open}
  <div class="when" role="group" aria-label="Conditions">
    <span class="lbl">Channels</span>
    <span class="ctl">
      <input
        class="field"
        aria-label="Channels"
        placeholder="Any channel"
        spellcheck="false"
        value={w.channels.join(', ')}
        onchange={(e) => typed(e, splitChannels, 'channels')}
      />
      {#if w.channels.length}
        <Segmented
          label="Channel scope"
          neutral
          options={SCOPE}
          value={w.notChannels ? 'never' : 'only'}
          onchange={(v) => set({ notChannels: v === 'never' })}
        />
      {/if}
    </span>

    <span class="lbl">Categories</span>
    <span class="ctl">
      <input
        class="field"
        aria-label="Categories"
        placeholder="Any, or Just Chatting, Minecraft"
        spellcheck="false"
        value={w.categories.join(', ')}
        onchange={(e) => typed(e, splitCategories, 'categories')}
      />
    </span>

    <span class="lbl">Stream</span>
    <span class="ctl">
      <Segmented
        label="Stream"
        neutral
        options={LIVE}
        value={w.live ?? 'any'}
        onchange={(v) => set({ live: v === 'any' ? null : (v as LiveWhen) })}
      />
    </span>

    <span class="lbl">You’re</span>
    <span class="ctl">
      <Segmented
        label="Your role there, or higher"
        neutral
        options={ROLE}
        value={w.role ?? 'any'}
        onchange={(v) => set({ role: v === 'any' ? null : (v as RoleWhen) })}
      />
      <span class="hint">or higher</span>
    </span>

    <span class="lbl">Burst</span>
    <span class="ctl">
      <NumberField
        label="Messages from one person"
        min={0}
        max={50}
        width="64px"
        value={w.burst}
        onchange={(n) => set({ burst: n < 2 ? 0 : n, burstSecs: n < 2 ? 0 : w.burstSecs || 10 })}
      />
      <!-- V110-F35 Off reads "Off · 2 or more turns it on", not a bare 0. -->
      {#if w.burst < 2}
        <span class="hint">Off · 2 or more turns it on</span>
      {:else}
        <span class="hint">messages in</span>
        <NumberField
          label="Seconds"
          min={1}
          max={600}
          unit="s"
          width="72px"
          value={w.burstSecs || 10}
          onchange={(n) => set({ burstSecs: n })}
        />
      {/if}
    </span>

    <span class="lbl"></span>
    <span class="ctl">
      <Checkbox label="First message only" checked={w.firstOnly} onchange={(c) => set({ firstOnly: c })} />
      {#if summary}<Button small variant="link" onclick={() => onchange(null)}>Clear</Button>{/if}
    </span>
  </div>
{:else if summary}
  <button type="button" class="chip" aria-label={`Conditions: ${summary}`} onclick={onopen}>{summary}</button>
{/if}

<style>
  .when {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 6px 12px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .lbl {
    color: var(--dim);
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    white-space: nowrap;
  }
  .ctl {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .field {
    flex: 1 1 160px;
    min-width: 0;
    height: 26px;
    text-overflow: ellipsis;
  }
  .hint {
    color: var(--dim);
    font-size: var(--text-hint);
  }
  .chip {
    max-width: 100%;
    height: 22px;
    padding: 0 8px;
    overflow: hidden;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: var(--text-hint);
    white-space: nowrap;
    text-overflow: ellipsis;
    transition: color var(--fast);
  }
  .chip:hover {
    color: var(--text);
  }
  .chip:active {
    opacity: 0.6;
    transition: none;
  }
</style>
