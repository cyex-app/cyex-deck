<!-- Commands (1.8): your own commands in Chatty's syntax, your menu entries and usercard
     buttons (S1), and timers waiting to run (S3). -->
<script lang="ts">
  import IconButton from '../../../components/IconButton.svelte';
  import Row from '../../../components/Row.svelte';
  import type { CommandLineError } from '../../../ipc/bindings/CommandLineError';
  import type { Commands } from '../../../ipc/bindings/Commands';
  import { rulesIpc } from '../../../ipc/rules';
  import { formatDate, formatTime } from '../../../lib/format';
  import { settings } from '../../../lib/settings.svelte';

  const c = $derived(settings.v.commands);
  const save = (patch: Partial<Commands>) => void settings.patch('commands', patch);

  let errors = $state.raw<CommandLineError[]>([]);

  $effect(() => {
    const lines = c.lines;
    rulesIpc
      .checkCommands(lines)
      .then((e) => (errors = e))
      .catch(() => {});
  });

  /** Textarea text → lines, without trailing blank ones. */
  function lines(text: string): string[] {
    const out = text.split('\n').map((l) => l.trimEnd());
    while (out.length && !out.at(-1)) out.pop();
    return out;
  }

  const MENUS: { key: 'userMenu' | 'messageMenu' | 'channelMenu' | 'usercardButtons'; label: string; hint: string }[] = [
    { key: 'userMenu', label: 'Name menu', hint: 'Right-click on a name.' },
    { key: 'messageMenu', label: 'Message menu', hint: 'Right-click on a message.' },
    { key: 'channelMenu', label: 'Chat menu', hint: 'The ⋯ button in a chat’s header.' },
    { key: 'usercardButtons', label: 'Usercard buttons', hint: 'Buttons on a usercard.' },
  ];

  const when = (ms: number) => `${formatDate(new Date(ms))} ${formatTime(new Date(ms))}`;
</script>

<h2>Commands</h2>

<Row
  setting="commands.lines"
  label="Your commands"
  hint="One per line: /name what it sends. $1 is the first word after it, $1- the rest, $(nick) the name it was used on. # starts a comment."
  stacked
>
  <textarea
    class="field code"
    rows="8"
    spellcheck="false"
    placeholder={'/hi /me waves at $1\n/so /shoutout $1'}
    value={c.lines.join('\n')}
    onchange={(e) => save({ lines: lines(e.currentTarget.value) })}
  ></textarea>
  {#each errors as e (e.line)}
    <p class="problem">Line {e.line + 1}: {e.message}</p>
  {/each}
</Row>

<h3 class="label">Menus</h3>
<p class="note">One per line: <code>Label=command</code>. <code>@Name</code> starts a submenu, <code>.Label=command</code> goes in it, <code>-</code> draws a line.</p>
{#each MENUS as m (m.key)}
  <Row setting="commands.{m.key}" label={m.label} hint={m.hint} stacked>
    <textarea
      class="field code"
      rows="3"
      spellcheck="false"
      placeholder={'Shoutout=/shoutout $(nick)\nLogs=/logs $(nick)'}
      value={c[m.key].join('\n')}
      onchange={(e) => save({ [m.key]: lines(e.currentTarget.value) })}
    ></textarea>
  </Row>
{/each}

<h3 class="label" data-setting="commands.timers">Timers</h3>
{#each c.timers as t (t.id)}
  <Row label={t.command} hint="{when(t.atMs)} in {t.channel}">
    <IconButton
      icon="trash"
      label="Cancel timer"
      size={14}
      onclick={() => save({ timers: c.timers.filter((x) => x.id !== t.id) })}
    />
  </Row>
{:else}
  <p class="empty">None waiting. /timer -r 10m /me stretch sets one that survives a restart.</p>
{/each}

<style>
  .field {
    width: 100%;
  }
  .code {
    height: auto;
    padding: 6px 8px;
    font-family: var(--font-mono);
    font-size: 0.8rem;
    resize: vertical;
  }
  .problem {
    margin: 0;
    color: var(--red);
    font-size: 0.78rem;
  }
  .note {
    margin: 0 0 4px;
    color: var(--muted);
    font-size: 0.85rem;
  }
  code {
    padding: 0 4px;
    background: var(--surface);
    border-radius: var(--radius);
    font-size: 0.8rem;
  }
  .empty {
    margin: 6px 0;
    color: var(--dim);
    font-size: 0.85rem;
  }
</style>
