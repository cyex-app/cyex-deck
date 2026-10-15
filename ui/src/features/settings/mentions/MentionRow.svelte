<!-- V104-01 One mention: what it looks for and how (whole word, case, look-alikes, regex
     and its flavor), then what it does (color, the Mentions tab, flash, pop-up, sound or
     its own sound file). "Your name" has only the second part. Errors and notes on the
     pattern show underneath. -->
<script lang="ts">
  import Checkbox from '../../../components/Checkbox.svelte';
  import IconButton from '../../../components/IconButton.svelte';
  import type { MentionAlert } from '../../../ipc/bindings/MentionAlert';
  import type { MentionCheck } from '../../../ipc/bindings/MentionCheck';
  import type { MentionRule } from '../../../ipc/bindings/MentionRule';
  import type { RegexFlavor } from '../../../ipc/bindings/RegexFlavor';
  import { mentionsIpc } from '../../../ipc/mentions';
  import { withAlpha } from '../rules/fields';
  import SoundFile from './SoundFile.svelte';

  type Item = MentionAlert & { enabled: boolean } & Partial<MentionRule>;

  interface Props {
    item: Item;
    /** "Your name": no pattern or match options. */
    title?: string;
    check?: MentionCheck;
    onchange: (item: Item) => void;
    ondelete?: () => void;
    /** V103-13 Registry id: settings search jumps here. */
    setting?: string;
  }

  let { item, title, check, onchange, ondelete, setting }: Props = $props();

  const FLAVORS: { value: RegexFlavor; label: string }[] = [
    { value: 'rust', label: 'Rust' },
    { value: 'javascript', label: 'JavaScript' },
    { value: 'python', label: 'Python' },
  ];

  function set<K extends keyof Item>(key: K, value: Item[K]) {
    if (item[key] === value) return;
    onchange({ ...item, [key]: value });
  }

  async function pickSound() {
    const path = await mentionsIpc.pickSound();
    if (path) onchange({ ...item, soundFile: path, sound: true });
  }
</script>

<div class="mention" class:off={!item.enabled} data-setting={setting}>
  <div class="line">
    <Checkbox label={title ?? 'On'} checked={item.enabled} onchange={(v) => set('enabled', v)} />
    {#if !title}
      <input
        class="field pattern"
        class:mono={item.regex}
        aria-label="Words or a regex"
        placeholder={item.regex ? 'A regex' : 'Words'}
        spellcheck="false"
        value={item.pattern}
        onchange={(e) => set('pattern', e.currentTarget.value)}
      />
    {/if}
    {#if ondelete}<IconButton icon="trash" label="Remove" size={14} onclick={ondelete} />{/if}
  </div>
  {#if !title}
    <div class="line opts">
      <Checkbox label="Whole word" checked={!!item.wholeWord} onchange={(v) => set('wholeWord', v)} />
      <Checkbox label="Match case" checked={!!item.caseSensitive} onchange={(v) => set('caseSensitive', v)} />
      <Checkbox label="Look-alikes" checked={!!item.lookalikes} onchange={(v) => set('lookalikes', v)} />
      <Checkbox label="Regex" checked={!!item.regex} onchange={(v) => set('regex', v)} />
      {#if item.regex}
        <select
          class="field"
          aria-label="Regex flavor"
          value={item.flavor}
          onchange={(e) => set('flavor', e.currentTarget.value as RegexFlavor)}
        >
          {#each FLAVORS as f (f.value)}<option value={f.value}>{f.label}</option>{/each}
        </select>
      {/if}
    </div>
  {/if}
  <div class="line opts">
    <span class="color" title={item.color ? 'Color' : 'Color: default'}>
      <span class="swatch" style:background={item.color ? item.color.slice(0, 7) : 'var(--accent)'}>
        <input
          type="color"
          aria-label="Color"
          value={item.color.slice(0, 7) || '#145aff'}
          onchange={(e) => set('color', withAlpha(e.currentTarget.value, item.color))}
        />
      </span>
      {#if item.color}<button class="clear" aria-label="Default color" onclick={() => set('color', '')}>×</button>{/if}
    </span>
    <Checkbox label="Show in Mentions" checked={item.show} onchange={(v) => set('show', v)} />
    <Checkbox label="Flash" checked={item.flash} onchange={(v) => set('flash', v)} />
    <Checkbox label="Pop-up" checked={item.notify} onchange={(v) => set('notify', v)} />
    <Checkbox label="Sound" checked={item.sound} onchange={(v) => set('sound', v)} />
    {#if item.soundFile}
      <SoundFile path={item.soundFile} onclear={() => set('soundFile', '')} />
    {:else}
      <button class="link" onclick={pickSound}>Sound file…</button>
    {/if}
  </div>
  {#if check?.error}<p class="problem">{check.error}</p>{:else if check?.note}<p class="note">{check.note}</p>{/if}
</div>

<style>
  .mention {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
    font-size: 0.85rem;
  }
  .off .opts,
  .off .pattern {
    opacity: 0.6;
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 12px;
  }
  .field {
    height: 28px;
    min-width: 0;
  }
  .pattern {
    flex: 1 1 160px;
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 0.8rem;
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
  .link {
    padding: 0;
    background: none;
    border: 0;
    color: var(--muted);
    text-decoration: underline;
  }
  .link:hover {
    color: var(--text);
  }
  .problem,
  .note {
    margin: 0;
    font-size: 0.78rem;
  }
  .problem {
    color: var(--red);
  }
  .note {
    color: var(--dim);
  }
</style>
