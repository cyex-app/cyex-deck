<!-- V105-20 A new version is out: a badge before Settings, and a pop-over with what's
     new, "Download and update", "Later" (asks again at the next start) and "Skip this version".
     V1061 The pop-over opens by itself when a version is found. Rust checks GitHub; the file
     must match its SHA-256 and carry a valid signature before it runs. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Popover from '../../components/Popover.svelte';
  import { update } from './update.svelte';

  let open = $state(false);

  $effect(() => {
    void update.start();
  });

  const u = $derived(update.available);

  // V1061 A version found opens the pop-over by itself, once per version this run; not after "Later".
  let shown: string | null = null;
  $effect(() => {
    const v = u?.version;
    if (!v || v === shown || update.later) return;
    shown = v;
    open = true;
  });

  function later() {
    open = false;
    update.deferLater();
  }

  function main() {
    if (u?.canInstall) void update.install();
    else update.openNotes();
  }

  function skip() {
    open = false;
    void update.skip();
  }
</script>

{#if u}
  <div class="wrap">
    <IconButton
      icon="download"
      label="Update available"
      hint={`cyex deck ${u.version} is out.`}
      badge
      active={open}
      onclick={() => (open = !open)}
    />
    <Popover bind:open label="Update">
      <div class="panel">
        <div class="title">
          <b>cyex deck {u.version}</b>
          {#if u.prerelease}<span class="chip">Pre-release</span>{/if}
        </div>
        <span class="notes"><Button small variant="link" onclick={() => update.openNotes()}>What’s new</Button></span>
        {#if update.busy}
          <div class="bar"><div class="fill" style:width="{update.percent ?? 0}%"></div></div>
          <span class="status">{update.progress}</span>
        {/if}
        {#if update.error}<span class="error">{update.error}</span>{/if}
        {#if !update.busy}
          <div class="buttons">
            <Button variant="primary" small onclick={main}>{u.canInstall ? 'Download and update' : 'Open release page'}</Button>
            <Button variant="ghost" small onclick={later}>Later</Button>
          </div>
          <div class="skip"><Button variant="ghost" small onclick={skip}>Skip this version</Button></div>
        {/if}
      </div>
    </Popover>
  </div>
{/if}

<style>
  .wrap {
    position: relative;
  }
  .panel {
    width: 260px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .title b {
    font-weight: 600;
  }
  .chip {
    padding: 1px 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--dim);
    font-size: 0.72rem;
  }
  .notes {
    align-self: flex-start;
  }
  .bar {
    height: 4px;
    border-radius: var(--radius-sm);
    background: var(--surface);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
  }
  .status {
    color: var(--muted);
    font-size: 0.82rem;
  }
  .error {
    color: var(--red);
    font-size: 0.82rem;
  }
  .buttons {
    display: flex;
    gap: 8px;
  }
  .skip {
    display: flex;
    justify-content: flex-end;
  }
</style>
