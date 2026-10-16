<!-- One rule in a settings list: its fields side by side, wrapping when narrow. Text
     saves when the field loses focus. V105-16 One line while it fits. -->
<script lang="ts" generics="T extends Record<string, unknown>">
  import Checkbox from '../../../components/Checkbox.svelte';
  import IconButton from '../../../components/IconButton.svelte';
  import { mentionsIpc } from '../../../ipc/mentions';
  import SoundFile from '../mentions/SoundFile.svelte';
  import { splitList, withAlpha, type Field } from './fields';

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
    /** `flow` wraps freely. `grid` (V105-18) lines every row's columns up on a wide page and
     *  falls back to `stack`: the match on line one, what it does on line two under it. */
    layout?: 'flow' | 'grid' | 'stack';
    /** Least width of the match field before it pushes the options onto a new line. */
    mainMin?: string;
    /** Remove ends line two (after what it does) instead of line one. */
    delLow?: boolean;
  }

  let { item, fields, title, onchange, ondelete, problem, note, fallback = 'var(--red)', setting, layout = 'flow', mainMin = '160px', delLow = false }: Props = $props();

  const cells = $derived(layout !== 'flow');
  const shown = $derived(fields.filter((f) => !f.when || f.when(item)));
  const firstAlert = $derived(shown.findIndex((f) => f.alert));

  function set(key: keyof T, value: unknown) {
    if (item[key] === value) return;
    onchange({ ...item, [key]: value });
  }

  async function pickSound(key: keyof T) {
    const path = await mentionsIpc.pickSound();
    if (path) onchange({ ...item, [key]: path, sound: true });
  }

  const text = (v: unknown) => (Array.isArray(v) ? v.join(', ') : String(v ?? ''));
</script>

{#snippet cell(f: Field<T>)}
  {@const v = item[f.key]}
  {#if f.kind === 'check'}
    <Checkbox label={f.short ?? f.label} hint={f.short ? f.label : undefined} checked={!!v} onchange={(c) => set(f.key, c)} />
  {:else if f.kind === 'color'}
    <span class="color" title={v ? f.label : `${f.label}: default`}>
      <span class="swatch" style:background={v ? String(v).slice(0, 7) : fallback}>
        <input
          type="color"
          aria-label={f.label}
          value={String(v).slice(0, 7) || '#145aff'}
          onchange={(e) => set(f.key, withAlpha(e.currentTarget.value, String(v)))}
        />
      </span>
      {#if v}<button class="clear" aria-label="Default color" onclick={() => set(f.key, '')}>×</button>{/if}
    </span>
  {:else if f.kind === 'number'}
    <input
      class="field num"
      type="number"
      aria-label={f.label}
      title={f.label}
      value={Number(v)}
      style:width={f.width ?? '72px'}
      onchange={(e) => set(f.key, Math.max(0, Math.round(Number(e.currentTarget.value) || 0)))}
    />
  {:else if f.kind === 'select'}
    <select class="field" aria-label={f.label} value={String(v)} onchange={(e) => set(f.key, e.currentTarget.value)}>
      {#each f.options ?? [] as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
    </select>
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
    {#if v}
      <SoundFile path={String(v)} onclear={() => set(f.key, '')} />
    {:else}
      <IconButton icon="upload" label={f.label} hint="Plays this file instead of the chime." size={14} onclick={() => void pickSound(f.key)} />
    {/if}
  {:else}
    <input
      class="field"
      class:grow={!f.width}
      class:mono={f.mono}
      aria-label={f.label}
      title={f.hint ?? f.label}
      placeholder={f.placeholder}
      spellcheck="false"
      style:width={cells ? '100%' : f.width}
      value={text(v)}
      onchange={(e) => set(f.key, f.kind === 'list' ? splitList(e.currentTarget.value) : e.currentTarget.value)}
    />
  {/if}
{/snippet}

<div class="rule" class:off={item.enabled === false} class:cells class:grid={layout === 'grid'} class:low={delLow} data-setting={setting} style:--main-min={mainMin}>
  {#if title && !cells}<span class="title">{title}</span>{/if}
  {#each shown as f, i (f.key)}
    {#if cells}
      {#if f.alert && i === firstAlert}<span class="brk"></span><span class="indent"></span>{/if}
      <span
        class="cell k-{f.key}"
        class:main={f.main}
        style:grid-column="{f.col ?? 'auto'} / span {f.span ?? 1}"
        style:flex={f.width && f.kind === 'text' ? `0 0 ${f.width}` : undefined}
      >
        {@render cell(f)}
      </span>
      {#if title && i === 0}<span class="cell title main" style="grid-column: 2 / span 3">{title}</span>{/if}
    {:else}
      {@render cell(f)}
    {/if}
  {/each}
  {#if ondelete}
    {#if cells}
      <span class="cell del" style="grid-column: 11"><IconButton icon="trash" label="Remove" size={14} onclick={ondelete} /></span>
    {:else}
      <IconButton icon="trash" label="Remove" size={14} onclick={ondelete} />
    {/if}
  {/if}
  {#if problem}<p class="problem">{problem}</p>{:else if note}<p class="note">{note}</p>{/if}
</div>

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
  .rule.off > :not(:first-child) {
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
  }
  /* V105-16 Shrinks first, so the checkboxes stay on one line with it; wraps only when even
     this minimum doesn't fit. */
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
    padding: 0 4px;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .clear:hover {
    color: var(--text);
  }
  .problem,
  .note {
    flex-basis: 100%;
    margin: 0;
    font-size: 0.78rem;
  }
  .problem {
    color: var(--red);
  }
  .note {
    color: var(--dim);
  }

  /* V105-18 Two lines: on, the match and its options; then, under the match, what it does. */
  .cells {
    gap: 6px 10px;
  }
  .cell {
    display: flex;
    align-items: center;
    min-width: 0;
  }
  .cell.main {
    flex: 1 1 var(--main-min);
  }
  .cell.main > :global(.field) {
    width: 100%;
  }
  .cell.title {
    font-weight: 600;
  }
  .k-enabled {
    flex: 0 0 40px;
  }
  .brk {
    flex: 0 0 100%;
    height: 0;
  }
  .indent {
    flex: 0 0 40px;
  }
  .cell.del {
    margin-left: auto;
  }
  /* Two lines: Remove ends the match line, so a narrow page never wraps it onto a third. */
  .cells .brk,
  .cells .brk ~ .indent {
    order: 1;
  }
  .cells .brk ~ .cell {
    order: 2;
  }
  .cells .brk ~ .cell.del {
    order: 0;
  }
  .cells.low .brk ~ .cell.del {
    order: 2;
  }
  .cells .problem,
  .cells .note {
    order: 3;
  }
  .cell :global(.file) {
    max-width: 100%;
  }

  /* Narrow page, Remove on line two: tighter, so Regex's flavor still leaves two lines. */
  @container (max-width: 840px) {
    .cells.low {
      column-gap: 4px;
    }
    .cells.low .indent {
      display: none;
    }
    .cells.low .k-flavor :global(select) {
      width: 66px;
      padding-inline: 6px 2px;
    }
  }

  /* V105-18 Wide page: one line, the same columns in every highlight row. */
  @container (min-width: 1080px) {
    .grid {
      display: grid;
      grid-template-columns: 40px minmax(150px, 1fr) 62px 42px 50px 62px 80px 58px 82px 88px 28px;
      gap: 6px;
    }
    .grid .brk,
    .grid .indent {
      display: none;
    }
    .grid .cell.del {
      margin: 0;
    }
    /* One line: DOM order again, or Remove (column 11) pushes the rest onto a new row. */
    .grid .brk ~ .cell,
    .grid .problem,
    .grid .note {
      order: 0;
    }
    .grid .problem,
    .grid .note {
      grid-column: 1 / -1;
    }
  }
</style>
