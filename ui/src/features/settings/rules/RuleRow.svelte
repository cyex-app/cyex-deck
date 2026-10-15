<!-- One rule in a settings list: its fields side by side, wrapping when narrow. Text
     saves when the field loses focus. V105-16 One line while it fits. -->
<script lang="ts" generics="T extends Record<string, unknown>">
  import Checkbox from '../../../components/Checkbox.svelte';
  import IconButton from '../../../components/IconButton.svelte';
  import { mentionsIpc } from '../../../ipc/mentions';
  import { mentionSound } from '../../../lib/sound';
  import { fileName } from '../mentions/status';
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
    /** V103-13 Registry id: settings search jumps here. */
    setting?: string;
  }

  let { item, fields, title, onchange, ondelete, problem, fallback = 'var(--red)', setting }: Props = $props();

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

<div class="rule" class:off={item.enabled === false} data-setting={setting}>
  {#if title}<span class="title">{title}</span>{/if}
  {#each fields as f (f.key)}
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
        <span class="file" title={String(v)}>
          {fileName(String(v))}
          <IconButton icon="play" label="Play" size={12} onclick={() => void mentionSound(String(v))} />
          <IconButton icon="x" label="Use the chime" size={12} onclick={() => set(f.key, '')} />
        </span>
      {:else}
        <IconButton icon="upload" label={f.label} hint="Plays this file instead of the chime." size={14} onclick={() => void pickSound(f.key)} />
      {/if}
    {:else}
      <input
        class="field"
        class:grow={!f.width}
        aria-label={f.label}
        title={f.hint ?? f.label}
        placeholder={f.placeholder}
        spellcheck="false"
        style:width={f.width}
        value={text(v)}
        onchange={(e) => set(f.key, f.kind === 'list' ? splitList(e.currentTarget.value) : e.currentTarget.value)}
      />
    {/if}
  {/each}
  {#if ondelete}<IconButton icon="trash" label="Remove" size={14} onclick={ondelete} />{/if}
  {#if problem}<p class="problem">{problem}</p>{/if}
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
  .file {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    max-width: 180px;
    overflow: hidden;
    color: var(--muted);
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .file :global(.icon-btn) {
    width: 22px;
    height: 22px;
  }
  .problem {
    flex-basis: 100%;
    margin: 0;
    color: var(--red);
    font-size: 0.78rem;
  }
</style>
