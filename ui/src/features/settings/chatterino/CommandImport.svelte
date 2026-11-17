<!-- V111-06 Commands from Chatterino's commands.json: pick the file, see each command as it
     comes in (notes on what changed, names in use, what can't come and why), then import. The
     toast has Undo. Chatterino's message-menu commands can come along, off by default. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Checkbox from '../../../components/Checkbox.svelte';
  import Dialog from '../../../components/Dialog.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import type { CommandItem } from '../../../ipc/bindings/CommandItem';
  import type { CommandPreview } from '../../../ipc/bindings/CommandPreview';
  import type { Skipped } from '../../../ipc/bindings/Skipped';
  import { settingsIpc } from '../../../ipc/settings';
  import { errorText } from '../../../lib/errors';
  import { formatNumber } from '../../../lib/format';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import { headline, isConflict, limitText, outcome, resultText } from './commandImport';
  import { failedImport } from './report';

  const MAX_SKIPPED = 20;
  const MINE = [
    { value: 'keep', label: 'Keep mine' },
    { value: 'take', label: 'Use Chatterino’s' },
  ] as const;
  const BUILT_IN = [
    { value: 'keep', label: 'Keep cyex’s' },
    { value: 'take', label: 'Use Chatterino’s' },
  ] as const;

  let preview = $state<CommandPreview | null>(null);
  let busy = $state(false);
  let take = $state<Set<number>>(new Set());
  let menu = $state(false);

  const result = $derived(preview ? outcome(preview, take) : null);
  const conflicts = $derived(preview ? preview.items.filter(isConflict) : []);
  const canImport = $derived(!!result && result.added + result.replaced > 0);
  const limit = $derived(preview && result ? limitText(preview, result) : null);

  async function pick() {
    busy = true;
    try {
      const p = await settingsIpc.commandsPreview();
      if (!p) return;
      preview = p;
      take = new Set();
      menu = false;
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }

  function choose(i: CommandItem, v: 'keep' | 'take') {
    const next = new Set(take);
    if (v === 'take') next.add(i.id);
    else next.delete(i.id);
    take = next;
  }

  function all(v: 'keep' | 'take') {
    take = v === 'take' ? new Set(conflicts.map((i) => i.id)) : new Set();
  }

  async function run() {
    if (!preview) return;
    busy = true;
    // Undo puts both lists back as they are now.
    const c = settings.v.commands;
    const before = { lines: [...c.lines], messageMenu: [...c.messageMenu] };
    try {
      const r = await settingsIpc.commandsImport([...take], menu && !!result?.menu);
      const changed = r.added + r.replaced + r.menu > 0;
      const undo = () => void settings.patch('commands', before);
      toast.success(resultText(r), changed ? { action: { label: 'Undo', run: undo } } : undefined);
      preview = null;
    } catch (e) {
      toast.error(failedImport(e));
      close();
    } finally {
      busy = false;
    }
  }

  function close() {
    preview = null;
    void settingsIpc.commandsCancel().catch(() => {});
  }

  function rows(list: Skipped[]) {
    return { shown: list.slice(0, MAX_SKIPPED), more: Math.max(0, list.length - MAX_SKIPPED) };
  }
</script>

<Button small icon="upload" disabled={busy} onclick={pick}>Import from Chatterino</Button>

{#if preview && result}
  {@const s = rows(preview.skipped)}
  <Dialog title="Import commands" width={560} locked={busy} onsubmit={() => canImport && !busy && void run()} onclose={close}>
    <p class="head">{headline(preview)}</p>
    {#if conflicts.length > 1}
      <div class="all">
        <span class="why">Names in use:</span>
        <Button small variant="ghost" onclick={() => all('keep')}>Keep all current</Button>
        <Button small variant="ghost" onclick={() => all('take')}>Use all Chatterino’s</Button>
      </div>
    {/if}
    {#if preview.items.length}
      <ul class="items">
        {#each preview.items as i (i.id)}
          <li>
            <code>{i.row}</code>
            {#each i.notes as n, k (k)}<span class="why">{n}</span>{/each}
            {#if i.conflict === 'yours'}
              <span class="why">You have <code>{i.yours}</code></span>
            {:else if i.conflict === 'builtIn'}
              <span class="why">cyex has its own command with this name.</span>
            {/if}
            {#if isConflict(i)}
              <Segmented
                options={i.conflict === 'yours' ? MINE : BUILT_IN}
                value={take.has(i.id) ? 'take' : 'keep'}
                label={`${i.name}: name in use`}
                onchange={(v) => choose(i, v)}
              />
              <!-- V111-55 Say what the pick does: the count in the button was the only change. -->
              <span class="why">{take.has(i.id) ? (i.conflict === 'yours' ? 'Replaces yours.' : 'Chatterino’s runs instead of cyex’s.') : 'Not imported.'}</span>
            {/if}
            {#if i.menuNote}<span class="why">Not for the message menu: {i.menuNote}</span>{/if}
          </li>
        {/each}
      </ul>
    {/if}
    {#if result.menu}
      <Checkbox
        checked={menu}
        label={`Add ${formatNumber(result.menu)} to the message menu too`}
        hint="Chatterino showed these on right-click. They go under Commands."
        onchange={(v) => (menu = v)}
      />
    {/if}
    {#if limit}<p class="warn">{limit}</p>{/if}
    {#if preview.unread}<p class="why">Only the first 2.000 were read; {formatNumber(preview.unread)} more stay out.</p>{/if}
    {#if preview.skipped.length}
      <p class="head">{formatNumber(preview.skipped.length)} can’t be imported:</p>
      <ul class="skipped">
        {#each s.shown as k, n (n)}
          <li><code>{k.pattern}</code> <span class="why">{k.reason}</span></li>
        {/each}
      </ul>
      {#if s.more}<p class="why">and {formatNumber(s.more)} more</p>{/if}
    {/if}
    {#snippet footer()}
      <Button variant="ghost" disabled={busy} onclick={close}>Cancel</Button>
      <Button variant="primary" disabled={busy || !canImport} onclick={run}>{canImport ? `Import ${formatNumber(result.added + result.replaced)}` : 'Import'}</Button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .head {
    margin: 0 0 8px;
    font-weight: 600;
  }
  .why {
    display: block;
    margin: 2px 0;
    font-size: var(--text-hint);
    color: var(--dim);
  }
  p.why {
    margin: 6px 0;
  }
  .warn {
    margin: 6px 0;
    font-size: var(--text-hint);
    color: var(--red);
  }
  .all {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 8px;
  }
  .all .why {
    display: inline;
  }
  ul {
    margin: 6px 0 8px;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 6px;
    font-size: var(--text-sub);
  }
  .items {
    max-height: 320px;
    overflow-y: auto;
  }
  .items li {
    display: grid;
    gap: 2px;
    justify-items: start;
  }
  li {
    overflow-wrap: anywhere;
  }
  code {
    font-family: var(--font-mono);
    font-size: 0.78rem;
  }
</style>
