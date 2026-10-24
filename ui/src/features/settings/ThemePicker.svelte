<!-- V106-02 The theme row: Dark, Light and the built-ins as tiles; custom themes in their menu; theme code. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import type { Appearance } from '../../ipc/bindings/Appearance';
  import type { CustomTheme } from '../../ipc/bindings/CustomTheme';
  import type { Theme } from '../../ipc/bindings/Theme';
  import { accentFill } from '../../design/theme';
  import { themeIpc } from '../../ipc/theme';
  import { errorText } from '../../lib/errors';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import CustomThemesMenu from './CustomThemesMenu.svelte';
  import { accentOf, newThemeFrom, PLAIN_PREVIEW, type ThemeChoice } from './themes';
  import { radioKey } from '../../components/select';

  let { a, builtins, choice }: { a: Appearance; builtins: readonly CustomTheme[]; choice: ThemeChoice } = $props();

  const BASES: Theme[] = ['dark', 'light'];

  const preview = (t: CustomTheme) => {
    const b = PLAIN_PREVIEW[t.base];
    return { bg: t.colors.bg ?? b.bg, surface: t.colors.surface ?? b.surface, text: t.colors.text ?? b.text };
  };

  /** V106 ra-3 Swatches show the accent as the UI paints it: the live `--accent` for the theme
   *  in use, else the fill worked out for that theme's base (a raw #101010 vanishes on Dark). */
  let fills = $state<Record<string, string>>({});
  const fillKey = (accent: string, base: Theme) => `${base}|${accent}`;
  function fillOf(accent: string, base: Theme, inUse: boolean): string {
    return inUse ? 'var(--accent)' : (fills[fillKey(accent, base)] ?? accent);
  }
  $effect(() => {
    const want: [string, Theme][] = [
      ...BASES.map((b): [string, Theme] => [a.accent, b]),
      ...builtins.map((t): [string, Theme] => [accentOf(a, { kind: 'builtin', theme: t }), t.base]),
      ...a.themes.map((t): [string, Theme] => [t.accent, t.base]),
    ];
    for (const [accent, base] of want) {
      const key = fillKey(accent, base);
      if (key in fills) continue;
      void accentFill(accent, base).then((f) => (fills[key] = f));
    }
  });

  /** The one tile in the tab order: the checked one, else Dark (a custom theme is in use). */
  const stop = $derived(!a.themeId ? a.theme : builtins.some((t) => t.id === a.themeId) ? a.themeId : 'dark');

  /** Roving focus over the tiles: one tab stop, arrows move and pick. */
  function keys(e: KeyboardEvent) {
    const tiles = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLElement>('[role="radio"]')];
    const at = tiles.indexOf(document.activeElement as HTMLElement);
    const next = at < 0 ? null : radioKey(e, at, tiles.length);
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
      <!-- V106 a-8 Dark and Light show your accent too. -->
      <span class="pic" style:background={b.bg}
        ><i style:background={b.surface}></i><b style:background={b.text}></b><u style:background={fillOf(a.accent, base, !a.themeId && a.theme === base)}></u></span
      >
      <span class="label">{b.label}</span>
    </button>
  {/each}
  {#each builtins as t (t.id)}
    {@const p = preview(t)}
    <button class="theme" role="radio" aria-checked={a.themeId === t.id} tabindex={stop === t.id ? 0 : -1} onclick={() => pickId(t.id)}>
      <span class="pic" style:background={p.bg}
        ><i style:background={p.surface}></i><b style:background={p.text}></b><u style:background={fillOf(accentOf(a, { kind: 'builtin', theme: t }), t.base, a.themeId === t.id)}></u></span
      >
      <span class="label">{t.name}</span>
    </button>
  {/each}
</div>
<div class="tools">
  <CustomThemesMenu themes={a.themes} activeId={a.themeId} onpick={pickId} onnew={newTheme}
    fill={(t) => fillOf(t.accent, t.base, t.id === a.themeId)} />
  <span class="import">
    <span class="code-label">Theme code</span>
    <input class="field code" placeholder="Paste a code" aria-label="Theme code" bind:value={code} />
    <Button small disabled={!code.trim()} onclick={importCode}>Add</Button>
  </span>
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
    font-size: var(--text-hint);
    cursor: pointer;
  }
  /* V106 s2-9 Ellipsis needs a box that can shrink; the flex column alone can't. */
  .label {
    max-width: 100%;
    min-width: 0;
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
  /* V106 s2-1 A ring in text color: the pick shows whatever the accent is. */
  .theme[aria-checked='true'] .pic {
    outline: 2px solid var(--text);
    outline-offset: 1px;
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
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .import {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
    min-width: 0;
  }
  .code-label {
    color: var(--muted);
    font-size: 0.8rem;
    white-space: nowrap;
  }
  .code {
    flex: 0 1 180px;
    min-width: 0;
    height: 28px;
  }
  /* Forced colours: the preview keeps its own colours; the pick gets a Highlight frame. */
  @media (forced-colors: active) {
    .pic,
    .pic * {
      forced-color-adjust: none;
    }
    .theme[aria-checked='true'] {
      border-color: Highlight;
    }
    .theme[aria-checked='true'] .pic {
      outline-color: Highlight;
    }
  }
</style>
