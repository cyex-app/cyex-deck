<!-- V103-06 One panel's frame in the mod view and the stream manager: a title bar (dragged
     while the layout is unlocked, with a close button then; V107-R29 a pop-out button on hover
     where the layout allows it), optional tools on the right,
     then the body, which scrolls. `flush` drops the body's padding, for lists. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import { quietScroll } from '../lib/quietScroll';
  import { grabFrom, usePanel } from './panel';
  import IconButton from './IconButton.svelte';

  interface Props {
    title: string;
    tools?: Snippet;
    flush?: boolean;
    children: Snippet;
  }

  let { title, tools, flush = false, children }: Props = $props();

  const panel = usePanel();
  const ctl = $derived(panel?.());
  const grab = $derived(grabFrom(ctl?.grab));
</script>

<section class="card">
  {#if !ctl?.bare || tools}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header class:grab={!!grab} onpointerdown={grab}>
    {#if ctl?.bare}<span class="room"></span>{:else}<h3>{title}</h3>{/if}
    {#if tools}<div class="tools">{@render tools()}</div>{/if}
    {#if ctl?.popout}
      <span class="out" class:shown={ctl.quick}><IconButton icon="popout" label="Pop out {title}" hint="Opens it in its own window." size={13} onclick={ctl.popout} /></span>
    {/if}
    {#if ctl?.close}
      <IconButton icon="x" label="Close {title}" size={14} onclick={ctl.close} />
    {/if}
  </header>
  {/if}
  <div class="body" class:flush use:quietScroll>{@render children()}</div>
</section>

<style>
  .card {
    flex: 1;
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    min-height: 34px;
    padding: 4px 6px 4px 10px;
    border-bottom: 1px solid var(--border);
  }
  header.grab {
    cursor: grab;
  }
  /* V107-R29 Quiet until the header is hovered or reached by keyboard. */
  .out {
    display: flex;
    opacity: 0;
    transition: opacity var(--fast);
  }
  header:hover .out,
  .out.shown,
  .out:focus-within {
    opacity: 1;
  }
  /* The title keeps its words; tools wrap beside it. */
  h3 {
    flex: 1 0 auto;
    margin: 0;
    /* One chip row tall, so the title stays on the first row when the tools wrap. */
    line-height: 26px;
    font-size: 0.8rem;
    font-weight: 700;
    white-space: nowrap;
  }
  .room {
    flex: 1;
  }
  .tools {
    display: flex;
    flex: 0 1 auto;
    flex-wrap: wrap;
    justify-content: flex-end;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }
  /* A narrow card stacks: rows wrap, fields take the full width, so nothing clips.
     Secondary actions: put the inline button in `.wide-only` and a ⋯ button in
     `.narrow-only`; the card shows one of them. */
  .body :global(.wide-only),
  .body :global(.narrow-only) {
    display: contents;
  }
  .body :global(.narrow-only) {
    display: none;
  }
  @container (max-width: 320px) {
    .body :global(.line),
    .body :global(.row),
    .body :global(.top) {
      flex-wrap: wrap;
    }
    .body :global(input.field:not(.grow)),
    .body :global(.line > .select),
    .body :global(.row > .select) {
      flex: 1 1 100%;
      width: 100% !important;
    }
    /* A field beside a button keeps a usable width; the button drops below when it can't. */
    .body :global(input.field.grow) {
      flex: 1 1 140px;
    }
    .body :global(.btn) {
      max-width: 100%;
      height: auto;
      min-height: 27px;
      padding-block: 3px;
      white-space: normal;
      text-align: center;
    }
    .body :global(.wide-only) {
      display: none;
    }
    .body :global(.narrow-only) {
      display: contents;
    }
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .body.flush {
    padding: 0;
    gap: 0;
  }
  .body :global(.field) {
    background: var(--bg);
    min-width: 0;
  }
  /* A short panel scrolls; its fields and rows keep their height (the poll's question
     field was squashed flat). */
  .body > :global(.field),
  .body > :global(input.field:not(.grow)),
  .body > :global(.line) {
    flex: none;
  }
  /* The start button stays in reach while the choices above scroll. */
  .body > :global(.foot) {
    position: sticky;
    bottom: -10px;
    margin-bottom: -10px;
    padding: 6px 0 10px;
    background: var(--surface);
    border-top: 1px solid var(--border);
  }
  .body :global(.note) {
    margin: 0;
    font-size: 0.78rem;
    color: var(--muted);
  }
  .body :global(.line) {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
</style>
