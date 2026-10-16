<!-- U8 Editing one custom theme: its colors on top of dark or light, a wallpaper, and its
     share code. Changes save as you go; Rust nudges colors that would hurt readability. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import Row from '../../components/Row.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import type { CustomTheme } from '../../ipc/bindings/CustomTheme';
  import type { ThemeLook } from '../../ipc/bindings/ThemeLook';
  import type { Theme } from '../../ipc/bindings/Theme';
  import type { WallpaperArea } from '../../ipc/bindings/WallpaperArea';
  import type { WallpaperFit } from '../../ipc/bindings/WallpaperFit';
  import { themeIpc } from '../../ipc/theme';
  import { toast } from '../../lib/toast.svelte';
  import { COLOR_NAMES, adjustedNote, isLink } from './themes';
  import { errorText } from '../../lib/errors';

  interface Props {
    theme: CustomTheme;
    /** The theme as painted, for colors the theme doesn't set itself. */
    look: ThemeLook | null;
    onchange: (next: CustomTheme) => void;
    ondelete: () => void;
  }

  let { theme, look, onchange, ondelete }: Props = $props();

  const BASES: { value: Theme; label: string }[] = [
    { value: 'dark', label: 'Dark' },
    { value: 'light', label: 'Light' },
  ];
  const AREAS: { value: WallpaperArea; label: string }[] = [
    { value: 'window', label: 'Whole window' },
    { value: 'chat', label: 'Chat only' },
  ];
  const FITS: { value: WallpaperFit; label: string }[] = [
    { value: 'cover', label: 'Fill' },
    { value: 'contain', label: 'Fit' },
    { value: 'tile', label: 'Tile' },
  ];

  const w = $derived(theme.wallpaper);
  const shown = (key: string) => theme.colors[key] ?? look?.palette?.tokens[key] ?? '#000000';
  const note = $derived(adjustedNote(look?.palette?.adjusted ?? []));

  // Dragging a color picker fires many inputs; save at most every 120 ms.
  let pending: CustomTheme | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;
  function save(next: CustomTheme, now = false) {
    pending = next;
    clearTimeout(timer);
    const flush = () => {
      if (pending) onchange(pending);
      pending = null;
    };
    if (now) flush();
    else timer = setTimeout(flush, 120);
  }

  const setColor = (key: string, value: string) => save({ ...theme, colors: { ...theme.colors, [key]: value } });

  function resetColor(key: string) {
    const colors = { ...theme.colors };
    delete colors[key];
    save({ ...theme, colors }, true);
  }

  const setWall = (patch: Partial<CustomTheme['wallpaper']>, now = true) =>
    save({ ...theme, wallpaper: { ...theme.wallpaper, ...patch } }, now);

  async function pickPicture() {
    try {
      const name = await themeIpc.pickWallpaper();
      if (name) setWall({ source: name });
    } catch (e) {
      toast.error(errorText(e));
    }
  }

  function useLink(input: HTMLInputElement) {
    const v = input.value.trim();
    if (!v) return;
    if (!isLink(v)) {
      toast.error('Use an https link to a picture.');
      return;
    }
    setWall({ source: v });
    input.value = '';
  }

  async function copyCode() {
    try {
      await navigator.clipboard.writeText(await themeIpc.code(theme.id));
      toast.info(w.source && !isLink(w.source) ? 'Theme code copied. Your wallpaper file stays on this PC.' : 'Theme code copied.');
    } catch (e) {
      toast.error(errorText(e));
    }
  }
</script>

<div class="editor">
  <Row label="Name">
    <input
      class="field name"
      maxlength="40"
      value={theme.name}
      onchange={(e) => save({ ...theme, name: e.currentTarget.value }, true)}
    />
  </Row>

  <Row label="Based on" hint="Backgrounds stay on that side so everything stays readable.">
    <Segmented label="Based on" options={BASES} value={theme.base} onchange={(base) => save({ ...theme, base }, true)} />
  </Row>

  <div class="colors">
    {#each COLOR_NAMES as [key, label] (key)}
      <div class="color">
        <input
          type="color"
          class="swatch"
          aria-label={label}
          value={shown(key)}
          oninput={(e) => setColor(key, e.currentTarget.value)}
        />
        <span class="cname">{label}</span>
        {#if theme.colors[key]}
          <button class="reset" aria-label="Back to the {theme.base} color" onclick={() => resetColor(key)}>×</button>
        {/if}
      </div>
    {/each}
  </div>
  {#if note}<p class="note">{note}</p>{/if}

  <h3 class="label">Wallpaper</h3>
  <Row label="Picture" hint={w.source ? (isLink(w.source) ? w.source : 'A picture from this PC.') : 'From this PC or a link.'}>
    <Button small icon="image" onclick={pickPicture}>Pick a picture</Button>
    <input
      class="field link"
      placeholder="https://…"
      aria-label="Picture link"
      onchange={(e) => useLink(e.currentTarget)}
    />
    {#if w.source}<Button small variant="ghost" onclick={() => setWall({ source: '' })}>Remove</Button>{/if}
  </Row>
  {#if w.source}
    <Row label="Where">
      <Segmented label="Where" options={AREAS} value={w.area} onchange={(area) => setWall({ area })} />
    </Row>
    <Row label="Size">
      <Segmented label="Size" options={FITS} value={w.fit} onchange={(fit) => setWall({ fit })} />
    </Row>
    <Row label="Dim" hint="How much the background covers the picture. More is easier to read.">
      <input
        type="range"
        min="0"
        max="95"
        step="5"
        value={w.dim}
        aria-label="Dim"
        oninput={(e) => setWall({ dim: Number(e.currentTarget.value) }, false)}
      />
      <span class="num">{w.dim} %</span>
    </Row>
    <Row label="Blur">
      <input
        type="range"
        min="0"
        max="30"
        value={w.blur}
        aria-label="Blur"
        oninput={(e) => setWall({ blur: Number(e.currentTarget.value) }, false)}
      />
      <span class="num">{w.blur} px</span>
    </Row>
  {/if}

  <div class="actions">
    <Button small icon="copy" onclick={copyCode}>Copy theme code</Button>
    <ConfirmButton label="Delete theme" confirm="Click again to delete" icon="trash" variant="ghost" onconfirm={ondelete} />
  </div>
</div>

<style>
  .editor {
    margin-top: 6px;
    padding: 4px 14px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .name {
    width: 220px;
  }
  .link {
    width: 180px;
  }
  .colors {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 8px 14px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }
  .color {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .cname {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .swatch {
    flex: none;
    width: 30px;
    height: 26px;
    padding: 2px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  .swatch::-webkit-color-swatch-wrapper {
    padding: 0;
  }
  .swatch::-webkit-color-swatch {
    border: 0;
    border-radius: 2px;
  }
  .reset {
    padding: 0 4px;
    background: none;
    border: 0;
    color: var(--dim);
    cursor: pointer;
  }
  .reset:hover {
    color: var(--text);
  }
  .note {
    margin: 8px 0 0;
    color: var(--dim);
    font-size: 0.8rem;
  }
  h3 {
    margin: 18px 0 2px;
  }
  input[type='range'] {
    width: 160px;
    accent-color: var(--accent);
  }
  .num {
    min-width: 44px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .actions {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding-top: 12px;
  }
</style>
