<!-- V1041-02 What a Chatterino layout brings: tabs and channels, what stays out and why, and
     whether viewer tabs open too. V111-60 Shared by the tabs dialog and Settings › Import. -->
<script lang="ts">
  import Checkbox from '../../../components/Checkbox.svelte';
  import type { ImportReport } from '../../../ipc/bindings/ImportReport';
  import { accounts } from '../../../lib/accounts.svelte';
  import { counts, leftOut } from './report';

  interface Props {
    report: ImportReport;
    viewerTabs: boolean;
    /** V111-60 Inside Settings › Import the kind's own heading says it: no bold headline. */
    compact?: boolean;
  }

  let { report, viewerTabs = $bindable(), compact = false }: Props = $props();

  const account = $derived(accounts.current?.displayName || accounts.current?.login);
  const lines = $derived(leftOut(report));
</script>

<p class={compact ? 'note top' : 'headline'}>{report.tabs ? counts(report) : 'Nothing to import.'}</p>
{#each lines as line (line.label)}
  <p class="note"><span class="why">{line.label}:</span> {line.text}</p>
{/each}
{#if report.tabs > 0}
  <div class="line">
    <Checkbox checked={viewerTabs} label="Also open viewer tabs" onchange={(v) => (viewerTabs = v)} />
  </div>
  {#if account}<p class="note">Joined as {account}.</p>{/if}
{/if}

<style>
  /* rb-9 A note: the hint size. */
  .note {
    margin: 10px 0;
    font-size: var(--text-hint);
    color: var(--dim);
  }
  .note.top {
    margin-top: 0;
  }
  .headline {
    margin: 0;
    font-weight: 600;
  }
  .why {
    color: var(--muted);
    font-weight: 600;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 10px;
  }
</style>
