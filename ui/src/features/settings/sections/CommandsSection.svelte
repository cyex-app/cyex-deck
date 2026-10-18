<!-- Commands (1.8): your own commands, menu entries and usercard buttons as rows (V106-10),
     and timers waiting to run (S3). -->
<script lang="ts">
  import { tick } from 'svelte';

  import EmptyState from '../../../components/EmptyState.svelte';
  import IconButton from '../../../components/IconButton.svelte';
  import Row from '../../../components/Row.svelte';
  import type { CommandLineError } from '../../../ipc/bindings/CommandLineError';
  import type { CommandRow } from '../../../ipc/bindings/CommandRow';
  import type { Commands } from '../../../ipc/bindings/Commands';
  import { rulesIpc } from '../../../ipc/rules';
  import { formatDate, formatTime } from '../../../lib/format';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import { GUIDE } from '../commands/guide';
  import GuideText from '../commands/GuideText.svelte';
  import {
    type CommandView,
    checkLines,
    commandRow,
    commandView,
    hiddenBySubmenu,
    type MenuView,
    menuRow,
    menuView,
  } from '../commands/rows';
  import type { Field } from '../rules/fields';
  import RuleList from '../rules/RuleList.svelte';
  import SectionHead from '../SectionHead.svelte';

  const c = $derived(settings.v.commands);
  const save = (patch: Partial<Commands>) => void settings.patch('commands', patch);

  let errors = $state.raw<CommandLineError[]>([]);
  /** The check itself failed, so the list above may be out of date. */
  let failed = $state(false);

  $effect(() => {
    const lines = checkLines(c.lines);
    let stale = false;
    rulesIpc
      .checkCommands(lines)
      .then((e) => !stale && ((errors = e), (failed = false)))
      .catch(() => !stale && (failed = true));
    return () => (stale = true);
  });

  const commandFields: Field<CommandView>[] = [
    { key: 'enabled', label: 'On', kind: 'check' },
    { key: 'name', label: 'Name', kind: 'text', placeholder: 'Name', width: '140px', mono: true, lead: true },
    { key: 'sends', label: 'What it sends', kind: 'text', placeholder: 'What it sends', main: true, mono: true },
  ];
  const menuFields: Field<MenuView>[] = [
    { key: 'enabled', label: 'On', kind: 'check' },
    { key: 'label', label: 'Label', kind: 'text', placeholder: 'Label', width: '150px', lead: true },
    { key: 'command', label: 'Command', kind: 'text', placeholder: 'Command', main: true, mono: true },
  ];

  type MenuKey = 'userMenu' | 'messageMenu' | 'channelMenu' | 'usercardButtons';
  const MENUS: { key: MenuKey; label: string; hint: string }[] = [
    { key: 'userMenu', label: 'Name menu', hint: 'Right-click a name.' },
    { key: 'messageMenu', label: 'Message menu', hint: 'Right-click a message.' },
    { key: 'channelMenu', label: 'Chat menu', hint: 'The ⋯ button in a chat’s header.' },
  ];

  /** s3-2 Cancels at once; Undo puts it back where it was. */
  function cancelTimer(id: (typeof c.timers)[number]['id']) {
    const i = c.timers.findIndex((x) => x.id === id);
    const gone = c.timers[i];
    if (!gone) return;
    save({ timers: c.timers.filter((x) => x.id !== id) });
    // rb-6 The cursor goes to the timer now in its place, the one above, or the list.
    void tick().then(() => {
      const left = timersEl?.querySelectorAll<HTMLElement>('button');
      (left?.[Math.min(i, left.length - 1)] ?? timersEl)?.focus();
    });
    // Into the list as it is then: timers that ran meanwhile stay gone.
    const undo = () => save({ timers: [...c.timers.slice(0, i), gone, ...c.timers.slice(i)] });
    toast.success('Timer canceled.', { action: { label: 'Undo', run: undo } });
  }

  let timersEl = $state<HTMLDivElement>();

  const when = (ms: number) => `${formatDate(new Date(ms))} ${formatTime(new Date(ms))}`;
</script>

{#snippet menuList(key: MenuKey, add: string, gone: string)}
  {@const rows: CommandRow[] = c[key]}
  <RuleList
    items={rows.map(menuView)}
    fields={menuFields}
    layout="split"
    make={() => ({ enabled: true, label: '', command: '', raw: '' })}
    onchange={(v) => save({ [key]: v.map(menuRow) })}
    addLabel={add}
    deleted={gone}
    empty="Nothing added."
    emptyIcon="list"
    reorder={{ key: `commands.${key}`, label: (v, i) => v.label || `Row ${i + 1}` }}
    note={(_, i) => (hiddenBySubmenu(rows, i) ? 'Hidden: the submenu above is off.' : null)}
  />
{/snippet}

<SectionHead title="Commands" level={2} docs="commands">Your own chat commands, menu entries and buttons.</SectionHead>

<SectionHead title={GUIDE.commands.title} setting="commands.lines"><GuideText entry={GUIDE.commands} /></SectionHead>
<RuleList
  items={c.lines.map(commandView)}
  fields={commandFields}
  layout="split"
  make={() => ({ enabled: true, name: '', sends: '', raw: '' })}
  onchange={(v) => save({ lines: v.map(commandRow) })}
  addLabel="Add command"
  deleted="Command deleted."
  empty="No commands yet."
  emptyIcon="terminal"
  reorder={{ key: 'commands.lines', label: (v, i) => v.name || `Row ${i + 1}` }}
  problem={(_, i) => errors.find((e) => e.line === i)?.message ?? null}
/>
{#if failed}<p class="problem">Couldn’t check these lines.</p>{/if}

<SectionHead title={GUIDE.menus.title}><GuideText entry={GUIDE.menus} /></SectionHead>
{#each MENUS as m (m.key)}
  <div class="sub" data-setting="commands.{m.key}">
    <span class="sub-title">{m.label}</span><span class="sub-hint">{m.hint}</span>
  </div>
  {@render menuList(m.key, 'Add entry', 'Entry deleted.')}
{/each}

<SectionHead title={GUIDE.usercard.title} setting="commands.usercardButtons"><GuideText entry={GUIDE.usercard} /></SectionHead>
{@render menuList('usercardButtons', 'Add button', 'Button deleted.')}

<SectionHead title={GUIDE.timers.title} setting="commands.timers"><GuideText entry={GUIDE.timers} /></SectionHead>
<div class="timers" tabindex="-1" bind:this={timersEl}>
  {#each c.timers as t (t.id)}
    <Row label={t.command} hint="{when(t.atMs)} in {t.channel}">
      <IconButton
        icon="trash"
        label="Cancel timer"
        size={14}
        onclick={() => cancelTimer(t.id)}
      />
    </Row>
  {:else}
    <EmptyState align="start" icon="clock" text="None waiting. /timer -r 10m /me stretch sets one that survives a restart." />
  {/each}
</div>

<style>
  .timers:focus {
    outline: none;
  }
  .problem {
    margin: 0;
    color: var(--red);
    font-size: var(--text-hint);
  }
  .sub {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin: 14px 0 0;
    font-size: var(--text-sub);
  }
  .sub-title {
    font-weight: 600;
  }
  .sub-hint {
    color: var(--dim);
    font-size: var(--text-hint);
  }
</style>
