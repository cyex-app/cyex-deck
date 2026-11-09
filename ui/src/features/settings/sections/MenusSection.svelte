<!-- V110-U67 Menus: the right-click menus, one row each. Open one to see it as it looks and
     rearrange, rename, hide or add rows, make groups, and add entries of your own. -->
<script lang="ts">
  import { tick } from 'svelte';

  import Icon from '../../../components/Icon.svelte';
  import { settings } from '../../../lib/settings.svelte';
  import MenuEditor from '../menus/MenuEditor.svelte';
  import { isChanged } from '../menus/model.ts';
  import { menusView } from '../menus/place.svelte.ts';
  import { DEFS, defOf, sampleOf } from '../menus/source.ts';
  import ResetSection from '../ResetSection.svelte';
  import SectionHead from '../SectionHead.svelte';

  const def = $derived(menusView.id ? defOf(menusView.id) : undefined);
  const layouts = $derived(settings.v.menus.layouts);

  let list = $state<HTMLElement>();

  async function back() {
    const id = menusView.id;
    menusView.id = '';
    await tick();
    list?.querySelector<HTMLElement>(`[data-menu="${CSS.escape(id)}"]`)?.focus();
  }
</script>

{#if def}
  <MenuEditor {def} sample={sampleOf} onback={() => void back()} />
{:else}
  <SectionHead title="Menus" level={2} setting="menus.layouts">
    Right-click menus. Open one to rearrange it, rename or hide rows, and add groups or entries of your own.
  </SectionHead>
  <div class="menus" bind:this={list}>
    {#each DEFS as d (d.id)}
      <button class="menu-row" data-menu={d.id} data-setting="menus:{d.id}" onclick={() => (menusView.id = d.id)}>
        <span class="text">
          <span class="title">{d.label}{#if isChanged(d, layouts[d.id])}<span class="tag">Changed</span>{/if}</span>
          <span class="hint">{d.where}</span>
        </span>
        <Icon name="chevronRight" size={14} />
      </button>
    {/each}
  </div>
  <ResetSection partition="menus" />
{/if}

<style>
  .menus {
    display: flex;
    flex-direction: column;
    max-width: 720px;
    border-top: 1px solid var(--border);
  }
  .menu-row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 48px;
    padding: 6px 8px 6px 4px;
    background: none;
    border: 0;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    text-align: left;
  }
  .menu-row:hover {
    background: var(--row-hover);
    color: var(--text);
  }
  .menu-row:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }
  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .title {
    color: var(--text);
  }
  .hint {
    color: var(--dim);
    font-size: var(--text-hint);
  }
  .tag {
    margin-left: 8px;
    padding: 0 6px;
    background: var(--accent-dim);
    border-radius: var(--radius-sm);
    color: var(--accent-text);
    font-size: 0.72rem;
    font-weight: 600;
  }
</style>
