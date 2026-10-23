<!-- One rule in a settings list: its fields side by side, wrapping when narrow. Text
     saves when the field loses focus. V105-16 One line while it fits.
     V105-18 `split` (Highlights, Mentions): the switch, then what the rule looks for with its
     options as icons, then what it does, then Remove. When the page is too narrow for one
     line, what it does moves under the match as a whole. -->
<script lang="ts" generics="T extends Record<string, unknown>">
  import Checkbox from '../../../components/Checkbox.svelte';
  import IconButton from '../../../components/IconButton.svelte';
  import IconToggle from '../../../components/IconToggle.svelte';
  import { openMenuAt } from '../../../components/menu.svelte';
  import RuleGrip from './RuleGrip.svelte';
  import { tooltip } from '../../../components/tooltip.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import { mentionsIpc } from '../../../ipc/mentions';
  import { soundIpc } from '../../../ipc/sound';
  import { fileName } from '../mentions/status';
  import { splitList, withAlpha, type Field } from './fields';
  import Select from '../../../components/Select.svelte';
  import NumberField from '../../../components/NumberField.svelte';

  interface Props {
    item: T;
    fields: Field<T>[];
    /** Text before the fields, e.g. a built-in highlight's name. */
    title?: string;
    onchange: (item: T) => void;
    ondelete?: () => void;
    /** Shown under the row, e.g. why a pattern doesn't read. */
    problem?: string | null;
    /** CSS color an empty color field stands for. */
    fallback?: string;
    /** A quiet note under the row, when there is no problem. */
    note?: string | null;
    /** V103-13 Registry id: settings search jumps here. */
    setting?: string;
    /** `flow` wraps freely; `split` see above. */
    layout?: 'flow' | 'split';
    /** Least width of the match field before what the rule does moves under it. */
    mainMin?: string;
    /** Room for this many option icons, so rows with fewer still line up. */
    slots?: number;
    /** V106-10 The drag grip's accessible name, e.g. "Move /hi". No grip when unset. */
    handle?: string;
    /** V106-10 An empty grip-wide column, so fixed rows line up with rows that have a grip. */
    gripSlot?: boolean;
  }

  let {
    item,
    fields,
    title,
    onchange,
    ondelete,
    problem,
    note,
    fallback = 'var(--red)',
    setting,
    layout = 'flow',
    mainMin = '140px',
    slots = 0,
    handle,
    gripSlot = false,
  }: Props = $props();

  const split = $derived(layout === 'split');
  const shown = $derived(fields.filter((f) => !f.when || f.when(item)));
  const on = $derived(split ? shown.find((f) => f.key === 'enabled') : undefined);
  const looks = $derived(shown.filter((f) => f !== on && !f.alert && !f.wide));
  const does = $derived(shown.filter((f) => f.alert && !f.wide));
  const wide = $derived(shown.filter((f) => f.wide));
  const grab = $derived(!!handle || gripSlot);

  function set(key: keyof T, value: unknown) {
    if (item[key] === value) return;
    onchange({ ...item, [key]: value });
  }

  async function pickSound(key: keyof T) {
    const path = await mentionsIpc.pickSound();
    if (path) onchange({ ...item, [key]: path, sound: true });
  }

  function soundMenu(e: MouseEvent, key: keyof T, path: string) {
    openMenuAt(e, [
      { label: 'Play', icon: 'play', run: () => void soundIpc.play(path) },
      { label: 'Choose another file', icon: 'upload', run: () => void pickSound(key) },
      'separator',
      { label: 'Use the chime', icon: 'x', run: () => set(key, '') },
    ]);
  }

  const text = (v: unknown) => (Array.isArray(v) ? v.join(', ') : String(v ?? ''));
  const tip = (f: Field<T>) => (f.hint ? { label: f.label, hint: f.hint } : '');
</script>

{#snippet cell(f: Field<T>)}
  {@const v = item[f.key]}
  {#if f.spacer}
    <span class="spacer" aria-hidden="true"></span>
  {:else if f.kind === 'check' && f.icon}
    <IconToggle icon={f.icon} label={f.label} hint={f.hint} pressed={!!v} onchange={(c) => set(f.key, c)} />
  {:else if f.kind === 'check'}
    <Checkbox label={f.label} checked={!!v} onchange={(c) => set(f.key, c)} />
  {:else if f.kind === 'color'}
    <span class="color" use:tooltip={{ label: v ? f.label : `${f.label}: default`, hint: f.hint }}>
      <span class="swatch" style:background={v ? String(v).slice(0, 7) : fallback}>
        <input
          type="color"
          aria-label={f.label}
          value={String(v).slice(0, 7) || '#145aff'}
          onchange={(e) => set(f.key, withAlpha(e.currentTarget.value, String(v)))}
        />
      </span>
      <button class="clear" class:gone={!v} aria-label="Default color" tabindex={v ? 0 : -1} onclick={() => set(f.key, '')}>×</button>
    </span>
  {:else if f.kind === 'number'}
    <span class="num" use:tooltip={{ label: f.label, hint: f.hint }}>
      {#if f.caption}<span class="cap" aria-hidden="true">{f.caption}</span>{/if}
      <NumberField label={f.label} min={0} max={1e9} unit={f.unit} width={f.width ?? '72px'} value={Number(v)} onchange={(n) => set(f.key, n)} />
    </span>
  {:else if f.kind === 'select'}
    <span use:tooltip={tip(f)}>
      <Select label={f.label} width={f.width ?? '150px'} value={String(v)} options={f.options ?? []} onchange={(x) => set(f.key, x)} />
    </span>
  {:else if f.kind === 'area'}
    <textarea
      class="field area"
      rows="2"
      aria-label={f.label}
      placeholder={f.placeholder}
      spellcheck="false"
      value={text(v)}
      onchange={(e) => set(f.key, e.currentTarget.value)}
    ></textarea>
  {:else if f.kind === 'sound'}
    <!-- V105-18 One icon: picks a file; once there is one, its menu plays or clears it. -->
    <IconToggle
      icon={f.icon ?? 'music'}
      label={v ? fileName(String(v)) : f.label}
      hint={v ? 'Plays instead of the chime. Click to play it, pick another or go back to the chime.' : f.hint}
      pressed={!!v}
      onchange={() => {}}
      onclick={(e) => (v ? soundMenu(e, f.key, String(v)) : void pickSound(f.key))}
    />
  {:else}
    <input
      class="field"
      class:grow={!f.width}
      class:mono={f.mono}
      aria-label={f.label}
      placeholder={f.placeholder}
      spellcheck="false"
      style:width={split && f.main ? undefined : f.width}
      value={text(v)}
      onchange={(e) => set(f.key, f.kind === 'list' ? splitList(e.currentTarget.value) : e.currentTarget.value)}
      use:tooltip={tip(f)}
    />
  {/if}
{/snippet}

{#if split}
  <div
    class="rule split"
    class:off={item.enabled === false}
    class:noon={!on}
    class:grab
    data-setting={setting}
    style:--main-min={mainMin}
    style:--slots={slots}
  >
    <RuleGrip {handle} slot={gripSlot} />
    <span class="on">
      {#if on}<Toggle label={title ?? on.label} checked={!!item[on.key]} onchange={(c) => set(on.key, c)} />{/if}
    </span>
    <div class="body">
      <div class="looks">
        {#if title}<span class="title">{title}</span>{/if}
        {#each looks.filter((f) => f.lead) as f (f.key)}<span class="lead">{@render cell(f)}</span>{/each}
        {#each looks.filter((f) => f.main) as f (f.key)}<span class="main">{@render cell(f)}</span>{/each}
        {#if looks.some((f) => !f.main && !f.lead) || slots}
          <span class="opts">{#each looks.filter((f) => !f.main && !f.lead) as f (f.key)}{@render cell(f)}{/each}</span>
        {/if}
      </div>
      {#if does.length}
        <div class="does">
          {#each does as f (f.key)}{@render cell(f)}{/each}
        </div>
      {/if}
      {#if wide.length}
        <div class="wide">
          {#each wide as f (f.key)}{@render cell(f)}{/each}
        </div>
      {/if}
    </div>
    <span class="del">{#if ondelete}<IconButton icon="trash" label="Delete" size={14} onclick={ondelete} />{/if}</span>
    {#if problem}<p class="problem">{problem}</p>{:else if note}<p class="note">{note}</p>{/if}
  </div>
{:else}
  <div class="rule" class:off={item.enabled === false} data-setting={setting}>
    <RuleGrip {handle} slot={gripSlot} />
    {#if title}<span class="title">{title}</span>{/if}
    {#each shown as f (f.key)}{@render cell(f)}{/each}
    {#if ondelete}<IconButton icon="trash" label="Delete" size={14} onclick={ondelete} />{/if}
    {#if problem}<p class="problem">{problem}</p>{:else if note}<p class="note">{note}</p>{/if}
  </div>
{/if}

<style>
  .rule {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
    font-size: 0.85rem;
  }
  /* Q5 Off rows stay in place, dimmed; the grip and the switch stay bright. s2-6 So does
     the note under it, which is already quiet. */
  .rule.off > :not(:first-child):not(:global(.grip) + *):not(.note) {
    opacity: 0.6;
  }
  .title {
    flex: 0 0 190px;
    font-weight: 600;
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 0.8rem;
  }
  .field {
    height: 28px;
    min-width: 0;
    /* b-9 A long name ends in … while the field isn't focused. */
    text-overflow: ellipsis;
  }
  /* V105-16 Shrinks first, so the checkboxes stay on one line with it. */
  .grow {
    flex: 1 1 90px;
    min-width: 80px;
  }
  .rule :global(.check) {
    gap: 5px;
    white-space: nowrap;
  }
  .area {
    flex: 1 1 100%;
    height: auto;
    padding: 5px 8px;
    font-family: var(--font-mono);
    font-size: 0.8rem;
    resize: vertical;
  }
  .color {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  .swatch {
    position: relative;
    display: block;
    width: 26px;
    height: 22px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  .swatch input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: pointer;
  }
  .clear {
    width: 16px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .clear:hover {
    color: var(--text);
  }
  .clear.gone {
    visibility: hidden;
  }
  .problem,
  .note {
    flex-basis: 100%;
    margin: 0;
    font-size: var(--text-hint);
  }
  .problem {
    color: var(--red);
  }
  .note {
    color: var(--dim);
  }

  /* V105-18 Switch | match and what it does | Remove. The middle wraps as two blocks, so
     every row keeps the same columns, on one line or two. */
  .split {
    display: grid;
    grid-template-columns: 36px minmax(0, 1fr) 26px;
    gap: 6px 12px;
  }
  .split.noon {
    grid-template-columns: minmax(0, 1fr) 26px;
  }
  .split.noon > .on {
    display: none;
  }
  .split.noon .problem,
  .split.noon .note {
    grid-column: 1 / -1;
  }
  .split > .on,
  .split > .del {
    align-self: start;
    display: flex;
    align-items: center;
    height: 28px;
  }
  /* V106-10 Grip | switch | body | Remove. */
  .split.grab {
    grid-template-columns: 18px 36px minmax(0, 1fr) 26px;
  }
  .split.grab.noon {
    grid-template-columns: 18px minmax(0, 1fr) 26px;
  }
  .split.grab .problem,
  .split.grab .note {
    grid-column: 3 / -1;
  }
  .split.grab.noon .problem,
  .split.grab.noon .note {
    grid-column: 2 / -1;
  }
  /* One icon toggle (26px) and its gap. */
  .spacer {
    flex: none;
    width: 26px;
  }
  .lead {
    flex: none;
    display: flex;
  }
  .split > .del :global(.icon-btn) {
    width: 26px;
    height: 26px;
  }
  .split .problem,
  .split .note {
    grid-column: 2 / -1;
  }
  .body {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 18px;
    min-width: 0;
  }
  .looks {
    flex: 1 1 auto;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .main {
    flex: 1 1 auto;
    display: flex;
    width: 0;
    min-width: var(--main-min);
  }
  .main > :global(*) {
    flex: 1 1 auto;
    width: 100%;
  }
  /* What it does moves under it first; b-9 past that, an ellipsis. */
  .split .title {
    flex: 1 1 auto;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .opts,
  .does {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  /* Right-aligned, so Regex sits in the same place with or without Match case. */
  .main + .opts {
    justify-content: flex-end;
    min-width: calc(var(--slots) * 28px - 2px);
  }
  /* C2 Long fields (a response, a filter) get the line under the row. */
  .wide {
    flex: 1 1 100%;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  /* C3 A number with its name beside it. */
  .num {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .cap {
    color: var(--muted);
    font-size: var(--text-hint);
    white-space: nowrap;
  }
  .does {
    flex: none;
  }
  .split .field {
    height: 26px;
  }
  .split .field.area {
    height: auto;
  }
  .split .color {
    margin-right: 2px;
  }
  .does > .field {
    margin-left: 6px;
  }
</style>
