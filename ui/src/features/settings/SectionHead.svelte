<!-- V106-12 A page or block heading with a quiet subtitle line and an optional Docs link. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { systemIpc } from '../../ipc/system';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';
  import { type DocsAnchor, docsUrl } from './docsLinks';

  interface Props {
    title: string;
    /** 2: the page heading (h2). 3: a block label (h3). */
    level?: 2 | 3;
    docs?: DocsAnchor;
    /** V103-13 Registry ids for settings search, on the heading. */
    setting?: string;
    /** The subtitle. */
    children?: Snippet;
    /** U7 Buttons beside the page heading (level 2 only). */
    actions?: Snippet;
  }

  let { title, level = 3, docs, setting, children, actions }: Props = $props();

  function open(e: MouseEvent) {
    e.preventDefault();
    if (docs) systemIpc.openLink(docsUrl(docs)).catch((err) => toast.error(errorText(err)));
  }
</script>

{#if level === 2 && actions}
  <div class="head">
    <h2 data-setting={setting}>{title}</h2>
    <div class="acts">{@render actions()}</div>
  </div>
{:else if level === 2}<h2 data-setting={setting}>{title}</h2>{:else}<h3 class="label" data-setting={setting}>{title}</h3>{/if}
{#if children || docs}
  <p class="sub">
    {@render children?.()}{#if docs}
      <a class="docs" href={docsUrl(docs)} onclick={open} use:tooltip={{ label: 'Docs', hint: 'Opens cyex.app in your browser.' }}>Docs</a>{/if}
  </p>
{/if}

<style>
  h3 {
    margin: 22px 0 2px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .acts {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  h2 + .sub,
  .head + .sub {
    margin: -8px 0 12px;
  }
  .sub {
    margin: 0 0 8px;
    max-width: 64ch;
    color: var(--muted);
    font-size: 0.85rem;
    line-height: 1.45;
  }
  .docs {
    color: var(--muted);
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
    text-underline-offset: 2px;
    white-space: nowrap;
  }
  .docs:hover {
    color: var(--text);
    text-decoration-color: currentColor;
  }
</style>
