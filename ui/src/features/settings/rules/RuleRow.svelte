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
  import Toggle from '../../../components/Toggle.svelte';
  import { tooltip } from '../../../components/tooltip.svelte';
  import { mentionsIpc } from '../../../ipc/mentions';
  import { soundIpc } from '../../../ipc/sound';
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
  }

  let { item, fields, title, onchange, ondelete, problem, note, fallback = 'var(--red)', setting, layout = 'flow', mainMin = '140px', slots = 0 }: Props = $props();

  const split = $derived(layout === 'split');
  const shown = $derived(fields.filter((f) => !f.when || f.when(item)));
  const on = $derived(split ? shown.find((f) => f.key === 'enabled') : undefined);
  const looks = $derived(shown.filter((f) => f !== on && !f.alert));
  const does = $derived(shown.filter((f) => f.alert));

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
  {#if f.kind === 'check' && f.icon}
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
    <select class="field" style:width={f.width} aria-label={f.label} value={String(v)} onchange={(e) => set(f.key, e.currentTarget.value)} use:tooltip={tip(f)}>
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
  <div class="rule split" class:off={item.enabled === false} data-setting={setting} style:--main-min={mainMin} style:--slots={slots}>
    <span class="on">
      {#if on}<Toggle label={title ?? on.label} checked={!!item[on.key]} onchange={(c) => set(on.key, c)} />{/if}
    </span>
    <div class="body">
      <div class="looks">
        {#if title}<span class="title">{title}</span>{/if}
        {#each looks.filter((f) => f.main) as f (f.key)}<span class="main">{@render cell(f)}</span>{/each}
        {#if looks.some((f) => !f.main) || slots}
          <span class="opts">{#each looks.filter((f) => !f.main) as f (f.key)}{@render cell(f)}{/each}</span>
        {/if}
      </div>
      <div class="does">
        {#each does as f (f.key)}{@render cell(f)}{/each}
      </div>
    </div>
    <span class="del">{#if ondelete}<IconButton icon="trash" label="Remove" size={14} onclick={ondelete} />{/if}</span>
    {#if problem}<p class="problem">{problem}</p>{:else if note}<p class="note">{note}</p>{/if}
  </div>
{:else}
  <div class="rule" class:off={item.enabled === false} data-setting={setting}>
    {#if title}<span class="title">{title}</span>{/if}
    {#each shown as f (f.key)}{@render cell(f)}{/each}
    {#if ondelete}<IconButton icon="trash" label="Remove" size={14} onclick={ondelete} />{/if}
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
    font-size: 0.78rem;
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
  .split > .on,
  .split > .del {
    align-self: start;
    display: flex;
    align-items: center;
    height: 28px;
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
  /* Never cut: what it does moves under it instead. */
  .split .title {
    flex: 1 1 auto;
    white-space: nowrap;
  }
  .opts,
  .does {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  /* Right-aligned, so Regex sits in the same place with or without Match case. */
  .opts {
    justify-content: flex-end;
    min-width: calc(var(--slots) * 28px - 2px);
  }
  .does {
    flex: none;
  }
  .split .field {
    height: 26px;
  }
  .split select.field {
    padding-block: 0;
  }
  .opts select.field {
    margin-left: 2px;
    padding-inline: 6px 2px;
  }
  .split .color {
    margin-right: 2px;
  }
  .does > .field {
    margin-left: 6px;
  }
</style>
