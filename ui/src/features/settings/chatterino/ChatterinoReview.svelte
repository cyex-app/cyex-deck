<!-- V111-60 One review for everything from Chatterino (or one kind of it): a part per kind with
     its own switch, the kind's own review inside, then one Import. Each kind goes in through its
     own import call; one Undo puts back highlights, ignores, commands and nicknames (tabs stay
     open, close them like any tab). -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Checkbox from '../../../components/Checkbox.svelte';
  import Dialog from '../../../components/Dialog.svelte';
  import type { ChatterinoScan } from '../../../ipc/bindings/ChatterinoScan';
  import type { ImportKind } from '../../../ipc/bindings/ImportKind';
  import { settingsIpc } from '../../../ipc/settings';
  import { errorText } from '../../../lib/errors';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import { outcome } from './commandImport';
  import CommandsReview from './CommandsReview.svelte';
  import ExtrasReview from './ExtrasReview.svelte';
  import HighlightsReview from './HighlightsReview.svelte';
  import { type Done, doneText, extrasCount, highlightCanImport, KINDS, snapshot, usersUndo } from './importAll';
  import TabsReview from './TabsReview.svelte';

  interface Props {
    scan: ChatterinoScan;
    /** The kinds asked for, in order. */
    kinds: ImportKind[];
    onclose: () => void;
  }

  let { scan, kinds, onclose }: Props = $props();

  let busy = $state(false);
  let viewerTabs = $state(false);
  let choice = $state<'mine' | 'theirs'>('mine');
  let take = $state<Set<number>>(new Set());
  let menu = $state(true);

  const shown = $derived(KINDS.filter((k) => kinds.includes(k.kind)));
  const errorOf = (kind: ImportKind) => scan.errors.find((e) => e.kind === kind)?.text;

  function ready(kind: ImportKind): boolean {
    switch (kind) {
      case 'tabs':
        return !!scan.tabs && scan.tabs.tabs > 0;
      case 'highlights':
        return !!scan.highlights && highlightCanImport(scan.highlights, choice);
      case 'commands': {
        if (!scan.commands) return false;
        const r = outcome(scan.commands, take);
        return r.added + r.replaced > 0 || (menu && r.menu > 0);
      }
      case 'extras':
        return !!scan.extras && extrasCount(scan.extras) > 0;
    }
  }

  // Every kind starts on; one with nothing to bring shows unticked until it has (a pick inside).
  let on = $state<Record<ImportKind, boolean>>({ tabs: true, highlights: true, commands: true, extras: true });

  const picked = $derived(shown.filter((k) => on[k.kind] && ready(k.kind)).map((k) => k.kind));
  const single = $derived(shown.length === 1);
  const title = $derived(single ? `Import ${shown[0].label.toLowerCase()}` : 'Import from Chatterino');
  const undoNote = $derived(picked.includes('tabs') && picked.length > 1);

  async function run() {
    busy = true;
    const before = snapshot(settings.v.highlights, settings.v.commands, settings.v.chat);
    const d: Done = { tabs: 0, highlights: 0, commands: 0, extras: 0, failed: [] };
    const fail = (label: string, e: unknown) => d.failed.push(`${label}: ${errorText(e).replace(/\.$/, '')}.`);
    let commandsChanged = false;
    try {
      if (picked.includes('highlights')) {
        try {
          const r = await settingsIpc.highlightsImport(choice === 'mine');
          d.highlights = r.added + r.replaced;
        } catch (e) {
          fail('Highlights', e);
        }
      }
      if (picked.includes('extras')) {
        try {
          d.extras = extrasCount(await settingsIpc.extrasImport());
        } catch (e) {
          fail('Ignores, nicknames and filters', e);
        }
      }
      if (picked.includes('commands')) {
        try {
          const menuToo = !!scan.commands && menu && outcome(scan.commands, take).menu > 0;
          const r = await settingsIpc.commandsImport([...take], menuToo);
          d.commands = r.added + r.replaced;
          commandsChanged = r.added + r.replaced + r.menu > 0;
        } catch (e) {
          fail('Commands', e);
        }
      } else if (scan.commands) {
        void settingsIpc.commandsCancel().catch(() => {});
      }
      if (picked.includes('tabs')) {
        try {
          d.tabs = (await settingsIpc.chatterinoImport(viewerTabs)).tabs;
        } catch (e) {
          fail('Tabs', e);
        }
      }
      if (!settings.v.general.chatterinoOffered) void settings.patch('general', { chatterinoOffered: true });
      const undoable = d.highlights + d.extras > 0 || commandsChanged;
      const undo = () => {
        if (d.highlights + d.extras > 0) void settings.patch('highlights', before.highlights);
        if (commandsChanged) void settings.patch('commands', before.commands);
        if (d.extras > 0) void settings.patch('chat', { users: usersUndo(before.users, settings.v.chat.users) });
      };
      const text = doneText(d);
      if (d.failed.length && !(d.tabs + d.highlights + d.commands + d.extras)) toast.error(text);
      else toast.success(text, undoable ? { action: { label: 'Undo', run: undo } } : undefined);
    } finally {
      busy = false;
      onclose();
    }
  }

  function cancel() {
    if (scan.commands) void settingsIpc.commandsCancel().catch(() => {});
    onclose();
  }
</script>

<Dialog {title} width={580} locked={busy} onsubmit={() => picked.length && !busy && void run()} onclose={cancel}>
  {#each shown as k (k.kind)}
    {@const error = errorOf(k.kind)}
    <section class="kind" class:off={!on[k.kind] || !!error}>
      {#if !single}
        <div class="top">
          <Checkbox
            checked={on[k.kind] && !error && ready(k.kind)}
            disabled={!!error || !ready(k.kind) || busy}
            label={k.label}
            onchange={(v) => (on[k.kind] = v)}
          />
        </div>
      {/if}
      {#if error}
        <p class="why">{error}</p>
      {:else if !single && !on[k.kind]}
        <p class="why">Left out.</p>
      {:else if k.kind === 'tabs' && scan.tabs}
        <TabsReview report={scan.tabs} bind:viewerTabs compact={!single} />
      {:else if k.kind === 'highlights' && scan.highlights}
        <HighlightsReview preview={scan.highlights} bind:choice />
      {:else if k.kind === 'commands' && scan.commands}
        <CommandsReview preview={scan.commands} bind:take bind:menu />
      {:else if k.kind === 'extras' && scan.extras}
        <ExtrasReview preview={scan.extras} />
      {/if}
    </section>
  {/each}
  {#if undoNote}<p class="why note">Undo puts back everything but the tabs.</p>{/if}
  {#snippet footer()}
    <Button variant="ghost" disabled={busy} onclick={cancel}>Cancel</Button>
    <Button variant="primary" disabled={busy || !picked.length} onclick={run}>Import</Button>
  {/snippet}
</Dialog>

<style>
  .kind {
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }
  .kind:first-child {
    padding-top: 0;
  }
  .kind:last-of-type {
    border-bottom: 0;
  }
  .top {
    margin-bottom: 8px;
  }
  .kind.off .top {
    margin-bottom: 0;
  }
  .why {
    margin: 6px 0 0;
    font-size: var(--text-hint);
    color: var(--dim);
  }
  .note {
    margin-top: 10px;
  }
</style>
