<!-- V106-02 The theme row: Dark, Light and the built-ins as tiles; custom themes in their menu; theme code. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import type { Appearance } from '../../ipc/bindings/Appearance';
  import type { CustomTheme } from '../../ipc/bindings/CustomTheme';
  import type { Theme } from '../../ipc/bindings/Theme';
  import { themeIpc } from '../../ipc/theme';
  import { errorText } from '../../lib/errors';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import CustomThemesMenu from './CustomThemesMenu.svelte';
  import { accentOf, newThemeFrom, PLAIN_PREVIEW, type ThemeChoice } from './themes';
  import { moveFor } from '../../components/select';

  let { a, builtins, choice }: { a: Appearance; builtins: readonly CustomTheme[]; choice: ThemeChoice } = $props();

  const BASES: Theme[] = ['dark', 'light'];

  const preview = (t: CustomTheme) => {
    const b = PLAIN_PREVIEW[t.base];
    return { bg: t.colors.bg ?? b.bg, surface: t.colors.surface ?? b.surface, text: t.colors.text ?? b.text };
  };

  /** The one tile in the tab order: the checked one, else Dark (a custom theme is in use). */
  const stop = $derived(!a.themeId ? a.theme : builtins.some((t) => t.id === a.themeId) ? a.themeId : 'dark');

  /** Roving focus over the tiles: one tab stop, arrows move and pick. */
  function keys(e: KeyboardEvent) {
    const tiles = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLElement>('[role="radio"]')];
    const at = tiles.indexOf(document.activeElement as HTMLElement);
    const key = e.key === 'ArrowRight' ? 'ArrowDown' : e.key === 'ArrowLeft' ? 'ArrowUp' : e.key;
    const next = at < 0 ? null : moveFor(key, at, tiles.length);
    if (next === null) return;
    e.preventDefault();
    tiles[next]?.focus();
    tiles[next]?.click();
  }

  const pickPlain = (theme: Theme) => settings.patch('appearance', { theme, themeId: '' });
  const pickId = (id: string) => settings.patch('appearance', { themeId: id });

  async function newTheme() {
    try {
      await themeIpc.add(newThemeFrom(a, choice));
    } catch (e) {
      toast.error(errorText(e));
    }
  }

  let code = $state('');
  async function importCode() {
    try {
      await themeIpc.import(code);
      code = '';
      toast.success('Theme added.');
    } catch (e) {
      toast.error(errorText(e));
    }
  }
</script>

<!-- svelte-ignore a11y_interactive_supports_focus -->
<div class="themes" role="radiogroup" aria-label="Theme" onkeydown={keys}>
  {#each BASES as base (base)}
    {@const b = PLAIN_PREVIEW[base]}
    <button class="theme" role="radio" aria-checked={!a.themeId && a.theme === base}
      tabindex={stop === base ? 0 : -1} onclick={() => pickPlain(base)}>
      <span class="pic" style:background={b.bg}><i style:background={b.surface}></i><b style:background={b.text}></b></span>
      {b.label}
    </button>
  {/each}
  {#each builtins as t (t.id)}
    {@const p = preview(t)}
    <button class="theme" role="radio" aria-checked={a.themeId === t.id} tabindex={stop === t.id ? 0 : -1} onclick={() => pickId(t.id)}>
      <span class="pic" style:background={p.bg}
        ><i style:background={p.surface}></i><b style:background={p.text}></b><u style:background={accentOf(a, { kind: 'builtin', theme: t })}></u></span
      >
      {t.name}
    </button>
  {/each}
</div>
<div class="tools">
  <CustomThemesMenu themes={a.themes} activeId={a.themeId} onpick={pickId} onnew={newTheme} />
  <span class="code-label">Theme code</span>
  <input class="field code" placeholder="Paste a code" aria-label="Theme code" bind:value={code} />
  <Button small disabled={!code.trim()} onclick={importCode}>Add</Button>
</div>

<style>
  .themes {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .theme {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    width: 92px;
    padding: 6px;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.78rem;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .theme:hover {
    background: var(--row-hover);
  }
  .theme[aria-checked='true'] {
    border-color: var(--accent-border);
    color: var(--text);
  }
  .pic {
    position: relative;
    display: grid;
    place-items: center;
    width: 78px;
    height: 46px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .pic i {
    position: absolute;
    inset: 0 auto 0 0;
    width: 24px;
  }
  .pic b {
    position: absolute;
    left: 32px;
    right: 10px;
    top: 14px;
    height: 4px;
    border-radius: var(--radius-sm);
  }
  .pic u {
    position: absolute;
    left: 32px;
    width: 18px;
    top: 26px;
    height: 6px;
    border-radius: var(--radius-sm);
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .code-label {
    margin-left: auto;
    color: var(--muted);
    font-size: 0.8rem;
    white-space: nowrap;
  }
  .code {
    flex: 0 1 180px;
    height: 28px;
  }
</style>
