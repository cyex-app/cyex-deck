<!-- Commands (1.8): your own commands as rows (V106-10),
     and timers waiting to run (S3). -->
<script lang="ts">
  import { tick } from 'svelte';

  import Button from '../../../components/Button.svelte';
  import EmptyState from '../../../components/EmptyState.svelte';
  import IconButton from '../../../components/IconButton.svelte';
  import Row from '../../../components/Row.svelte';
  import type { CommandLineError } from '../../../ipc/bindings/CommandLineError';
  import type { Commands } from '../../../ipc/bindings/Commands';
  import { rulesIpc } from '../../../ipc/rules';
  import { formatDate, formatTime } from '../../../lib/format';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import { settingsJump } from '../menus/place.svelte';
  import { GUIDE } from '../commands/guide';
  import GuideText from '../commands/GuideText.svelte';
  import {
    type CommandView,
    checkLines,
    commandRow,
    commandProblem,
    commandView,
  } from '../commands/rows';
  import type { Field } from '../rules/fields';
  import RuleList from '../rules/RuleList.svelte';
  import { CAPS } from '../rules/caps';
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

<!-- V110-N6 Menu entries and usercard buttons moved to the Menus page; one line points there. -->
<SectionHead title="Commands" level={2} docs="commands">
  Your own chat commands and timers. Menu entries and usercard buttons are on the
  <Button variant="link" hit onclick={() => (settingsJump.to = 'menus')}>Menus page</Button>.
</SectionHead>

<SectionHead title={GUIDE.commands.title} setting="commands.lines"><GuideText entry={GUIDE.commands} /></SectionHead>
<RuleList
  items={c.lines.map(commandView)}
  max={CAPS.commandLines}
  fields={commandFields}
  layout="split"
  make={() => ({ enabled: true, name: '', sends: '', raw: '' })}
  onchange={(v) => save({ lines: v.map(commandRow) })}
  addLabel="Add command"
  deleted="Command deleted."
  empty="No commands yet."
  emptyIcon="terminal"
  reorder={{ key: 'commands.lines', label: (v, i) => v.name || `Row ${i + 1}` }}
  problem={(v, i) => errors.find((e) => e.line === i)?.message ?? commandProblem(v)}
/>
{#if failed}<p class="problem">Couldn’t check these lines.</p>{/if}

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
</style>
