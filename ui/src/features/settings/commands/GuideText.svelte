<!-- V106-13 A guide entry as plain text: intro, variables, a worked example, extra notes. -->
<script lang="ts">
  import type { GuideEntry } from './guide';

  let { entry }: { entry: GuideEntry } = $props();
</script>

{entry.intro}{#if entry.variables}
  {entry.variables}{/if}{#if entry.example.typed}
  <span class="ex"
    >{#each entry.example.row as part, i (i)}{#if i}
        <span class="sep">·</span>
      {/if}<code>{part}</code>{/each}{#if entry.example.row.length}
      <span class="sep">→</span>
    {/if}<code>{entry.example.typed}</code> gives <code>{entry.example.result}</code></span
  >{/if}{#if entry.more}
  <span class="more">{entry.more}</span>{/if}

<style>
  .ex {
    display: block;
    margin-top: 4px;
  }
  code {
    font-family: var(--font-mono);
    font-size: 0.78rem;
    padding: 0 4px;
    background: var(--surface);
    border-radius: var(--radius-sm);
    color: var(--text);
  }
  .sep {
    color: var(--dim);
  }
  .more {
    display: block;
    margin-top: 2px;
    color: var(--dim);
    font-size: 0.8rem;
  }
</style>
