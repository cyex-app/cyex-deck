<!-- V111-04 Moderation: what used to sit under Chat → Moderation. Same settings, only the page changed. -->
<script lang="ts">
  import { enterKey } from '../../../lib/keys';
  import SectionHead from '../SectionHead.svelte';
  import DragList from '../../../components/DragList.svelte';
  import Icon from '../../../components/Icon.svelte';
  import type { IconName } from '../../../components/icons';
  import Row from '../../../components/Row.svelte';
  import type { ModButton } from '../../../ipc/bindings/ModButton';
  import type { ModButtons } from '../../../ipc/bindings/ModButtons';
  import { formatDuration, formatSecs } from '../../../lib/format';
  import { settings } from '../../../lib/settings.svelte';
  import { addTimeout, buttonKey, buttonOn, canAddButton, fullOrder, retimeButton, toggleButton } from '../../modview/lineMod';
  import { timeoutSecs } from '../../modview/timeoutLength';
  import ModePresets from './ModePresets.svelte';
  import { adSecs, followMinutes, slowSecs } from '../../modview/modePresets';
  import ResetSection from '../ResetSection.svelte';
  import Select from '../../../components/Select.svelte';

  const c = $derived(settings.v.chat);

  let newPreset = $state('');
  /** V107-R22 What a typed length was refused for, under its field. */
  let presetError = $state('');

  function addPreset() {
    const t = timeoutSecs(newPreset);
    if ('error' in t) {
      presetError = t.error;
      return;
    }
    // V110-F101 Say so, and keep the field.
    if (c.timeoutPresets.includes(t.secs)) {
      presetError = 'Already in the list.';
      return;
    }
    void settings.patch('chat', { timeoutPresets: [...c.timeoutPresets, t.secs] });
    newPreset = '';
  }

  function removePreset(secs: number) {
    void settings.patch('chat', { timeoutPresets: c.timeoutPresets.filter((s) => s !== secs) });
  }


  /** M10 Mod mode's buttons: your timeout lengths plus any already picked. */
  const mb = $derived(c.modButtons);
  const modTimes = $derived([...new Set([...c.timeoutPresets, ...mb.timeouts])].sort((a, b) => a - b));
  const flipMod = (patch: Partial<ModButtons>) => void settings.patch('chat', { modButtons: patch });

  function flipTime(secs: number) {
    const on = mb.timeouts.includes(secs);
    flipMod({ timeouts: on ? mb.timeouts.filter((s) => s !== secs) : [...mb.timeouts, secs] });
  }

  /** V105-15 The order list: every button with its place, as it looks in chat. */
  const modOrder = $derived(fullOrder(mb, modTimes));
  const modName = (b: ModButton) =>
    typeof b !== 'string' ? `Time out ${formatDuration(b.timeout)}` : b === 'delete' ? 'Delete' : b === 'warn' ? 'Warn' : b === 'unban' ? 'Unban' : 'Ban';
  const modIcon = (b: ModButton): IconName =>
    typeof b !== 'string' ? 'clock' : b === 'delete' ? 'trash' : b === 'warn' ? 'alert' : b === 'unban' ? 'undo' : 'gavel';
  const modOn = (b: ModButton) => buttonOn(mb, b);

  /** V107-R22 A timeout button typed in: any length Twitch takes. */
  let newButton = $state('');
  let buttonError = $state('');
  function addButton() {
    const t = timeoutSecs(newButton);
    const patch = 'error' in t ? t.error : addTimeout(mb, t.secs);
    if (typeof patch === 'string') {
      buttonError = patch;
      return;
    }
    flipMod(patch);
    newButton = '';
  }

  /** V107-R22 The timeout row being given another length, and what was typed. */
  let editing = $state<number | null>(null);
  let editText = $state('');
  let editError = $state('');
  function startEdit(secs: number) {
    editing = secs;
    editText = formatDuration(secs).replace(' ', '');
    editError = '';
  }
  function saveEdit() {
    if (editing === null) return;
    const from = editing;
    const t = timeoutSecs(editText);
    const patch = 'error' in t ? t.error : retimeButton(mb, from, t.secs, modTimes);
    if (typeof patch === 'string') {
      editError = patch;
      return;
    }
    editing = null;
    if ('error' in t || !patch.order) return; // the same length: nothing to save
    const to = t.secs;
    // A preset's own row changes the preset too, or the old length would come back as a row.
    const timeoutPresets = c.timeoutPresets.map((s) => (s === from ? to : s));
    void settings.patch('chat', { modButtons: patch, timeoutPresets });
  }
  /** Clicking away keeps a length that works, as Enter does; anything else is dropped. */
  function blurEdit() {
    if (editing === null) return;
    const t = timeoutSecs(editText);
    if ('error' in t || typeof retimeButton(mb, editing, t.secs, modTimes) === 'string') editing = null;
    else saveEdit();
  }
  function editKey(e: KeyboardEvent) {
    if (enterKey(e)) {
      e.preventDefault();
      saveEdit();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      editing = null;
    }
  }
  const focus = (el: HTMLInputElement) => {
    el.focus();
    el.select();
  };
</script>

<SectionHead title="Moderation" level={2} page="moderation" />

<Row setting="chat.timeoutPresets" label="Timeout buttons" hint="Quick durations in chat and on user cards. Up to 8." stacked>
  {#each c.timeoutPresets as secs (secs)}
    <span class="chip">
      {formatDuration(secs)}
      <button aria-label="Remove {formatDuration(secs)}" onclick={() => removePreset(secs)}>×</button>
    </span>
  {/each}
  {#if c.timeoutPresets.length < 8}
    <form
      onsubmit={(e) => {
        e.preventDefault();
        addPreset();
      }}
    >
      <input
        class="field add"
        class:bad={!!presetError}
        aria-label="Add a timeout length"
        aria-invalid={!!presetError}
        placeholder="Add, e.g. 30m"
        bind:value={newPreset}
        oninput={() => (presetError = '')}
      />
    </form>
  {/if}
  {#if presetError}<span class="field-error" role="alert">{presetError}</span>{/if}
</Row>
<!-- V110-M23 Lengths for the mode menus and the ad picker in the stream manager. -->
<ModePresets
  setting="chat.slowPresets"
  label="Slow mode lengths"
  hint="Choices when you set slow mode. 3s to 2m."
  values={c.slowPresets}
  show={formatSecs}
  parse={slowSecs}
  placeholder="Add, e.g. 45s"
  onsave={(list) => void settings.patch('chat', { slowPresets: list })}
/>
<ModePresets
  setting="chat.followPresets"
  label="Followers-only ages"
  hint="Choices for how long someone must have followed. Any follower is always there. Up to 3 months."
  values={c.followPresets}
  show={(m) => formatSecs(m * 60)}
  parse={followMinutes}
  placeholder="Add, e.g. 2d"
  onsave={(list) => void settings.patch('chat', { followPresets: list })}
/>
<ModePresets
  setting="chat.adPresets"
  label="Ad lengths"
  hint="Choices for a commercial: 30s to 3m in steps of 30s."
  values={c.adPresets}
  show={formatSecs}
  parse={adSecs}
  placeholder="Add, e.g. 150s"
  max={6}
  onsave={(list) => void settings.patch('chat', { adPresets: list })}
/>
<Row setting="chat.adLength" label="Default ad length" hint="Picked when the stream manager opens.">
  <Select
    label="Default ad length"
    width="110px"
    value={String(c.adLength)}
    options={c.adPresets.map((v) => ({ value: String(v), label: formatSecs(v) }))}
    onchange={(v) => settings.patch('chat', { adLength: Number(v) })}
  />
</Row>
<Row setting="chat.modButtons" label="Mod mode buttons" hint="In front of lines while mod mode is on. Up to 4 timeouts." stacked>
  <button class="pick" aria-pressed={mb.delete} onclick={() => flipMod({ delete: !mb.delete })}><span class="cap-text">Delete</span></button>
  {#each modTimes as secs (secs)}
    {@const on = mb.timeouts.includes(secs)}
    <button class="pick" aria-pressed={on} disabled={!on && mb.timeouts.length >= 4}
      title={!on && mb.timeouts.length >= 4 ? '4 timeouts max' : undefined}
      onclick={() => flipTime(secs)}><span class="cap-text">{formatDuration(secs)}</span></button
    >
  {/each}
  <button class="pick" aria-pressed={mb.warn} onclick={() => flipMod({ warn: !mb.warn })}><span class="cap-text">Warn</span></button>
  <button class="pick" aria-pressed={mb.ban} onclick={() => flipMod({ ban: !mb.ban })}><span class="cap-text">Ban</span></button>
  <!-- V107-R22b Lifts a ban or timeout; off by default. -->
  <button class="pick" aria-pressed={mb.unban} onclick={() => flipMod({ unban: !mb.unban })}><span class="cap-text">Unban</span></button>
  {#if mb.timeouts.length >= 4 && modTimes.some((t) => !mb.timeouts.includes(t))}
    <span class="slot-hint" title="Remove a timeout to pick another">4 timeouts max</span>
  {:else if mb.timeouts.length < 4}
    <!-- V107-R22 Any length, straight onto a button. -->
    <form
      onsubmit={(e) => {
        e.preventDefault();
        addButton();
      }}
    >
      <input
        class="field add"
        class:bad={!!buttonError}
        aria-label="Add a timeout button"
        aria-invalid={!!buttonError}
        placeholder="Timeout, e.g. 7m"
        bind:value={newButton}
        oninput={() => (buttonError = '')}
      />
    </form>
  {/if}
  {#if buttonError}<span class="field-error" role="alert">{buttonError}</span>{/if}
</Row>
<Row setting="chat.modButtons.order" label="Button order" hint="Drag them into the order they show in chat, or Alt+↑ and Alt+↓." stacked>
  <div class="order">
    <DragList key="modButtons" items={modOrder} id={buttonKey} label={modName} onmove={(order) => flipMod({ order })}>
      {#snippet row(b)}
        <div class="slot" class:off={!modOn(b)}>
          <span class="grip"><Icon name="grip" size={14} /></span>
          <Icon name={modIcon(b)} size={14} />
          {#if typeof b !== 'string' && editing === b.timeout}
            <!-- V107-R22 Another length for this button, typed. -->
            <span class="slot-name">
              Time out
              <input
                class="field len"
                class:bad={!!editError}
                aria-label="Length"
                aria-invalid={!!editError}
                bind:value={editText}
                use:focus
                oninput={() => (editError = '')}
                onkeydown={editKey}
                onblur={blurEdit}
              />
              {#if editError}<span class="field-error" role="alert">{editError}</span>{/if}
            </span>
          {:else if typeof b !== 'string'}
            {@const secs = b.timeout}
            <span class="slot-name">
              Time out
              <button class="len-btn" data-tip="Change the length" aria-label="Change the length of {modName(b)}" onclick={() => startEdit(secs)}
                >{formatDuration(secs)}</button
              >
            </span>
          {:else}
            <span class="slot-name">{modName(b)}</span>
          {/if}
          {#if !modOn(b) && !canAddButton(mb, b)}<span class="slot-hint">4 timeouts max</span>{/if}
          {#if typeof b === 'string' || editing !== b.timeout}<button
            class="slot-act"
            disabled={!canAddButton(mb, b)}
            title={canAddButton(mb, b) ? undefined : '4 timeouts max'}
            onclick={() => flipMod(toggleButton(mb, b))}
            aria-label={`${modOn(b) ? 'Remove' : 'Add'} ${modName(b)}`}
            >{modOn(b) ? 'Remove' : 'Add'}</button
          >{/if}
        </div>
      {/snippet}
    </DragList>
  </div>
</Row>

<ResetSection partition="chat" page="moderation" />

<style>
  .order {
    width: 100%;
    max-width: 320px;
  }
  .slot {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 10px 0 6px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-weight: 600;
  }
  .slot.off {
    color: var(--dim);
  }
  .grip {
    display: grid;
    place-items: center;
    color: var(--dim);
  }
  .slot-name {
    flex: 1;
  }
  .slot-act {
    padding: 0 6px;
    background: none;
    border: 0;
    color: var(--dim);
    font: inherit;
    font-weight: 400;
    font-size: var(--text-hint);
    cursor: pointer;
  }
  .slot-act:hover:not(:disabled) {
    color: var(--text);
  }
  .slot-act:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .slot-hint {
    color: var(--dim);
    font-size: 0.74rem;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 28px;
    padding: 0 4px 0 10px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    font: 600 0.82rem var(--font-mono);
  }
  .chip button {
    position: relative;
    width: 20px;
    height: 20px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .chip button::before {
    content: "";
    position: absolute;
    inset: -3px;
  }
  .chip button:hover {
    color: var(--red);
  }
  .add {
    width: 130px;
    height: 28px;
  }
  /* V107-R22 Typed lengths: a refusal reads in red next to its field. */
  .field.bad,
  .field.bad:focus-visible {
    border-color: var(--red);
    box-shadow: 0 0 0 1px var(--red);
  }
  .field-error {
    white-space: nowrap;
    color: var(--red);
    font-size: 0.76rem;
    font-weight: 400;
  }
  .slot-name {
    display: flex;
    white-space: nowrap;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .len-btn {
    height: 22px;
    padding: 0 6px;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius);
    color: inherit;
    font: inherit;
    text-decoration: underline dotted var(--dim);
    text-underline-offset: 3px;
    cursor: text;
  }
  .len-btn:hover {
    background: var(--surface-hover);
    border-color: var(--border-strong);
    text-decoration: none;
  }
  .len {
    width: 72px;
    height: 22px;
    padding: 0 6px;
    font: inherit;
  }
  .pick {
    height: 28px;
    padding: 0 10px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--muted);
    font-weight: 600;
  }
  .pick:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text);
  }
  /* V110-F35 Picked: accent-dim fill and accent text (cyex-app), not a faint border. */
  .pick[aria-pressed='true'] {
    border-color: var(--accent-border);
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  .pick[aria-pressed='true']:hover:not(:disabled) {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  .pick:disabled {
    opacity: 0.5;
  }
</style>
