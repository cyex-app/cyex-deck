<!-- S5 A plugin's console: its log as it comes in, and a line that runs Lua inside the
     plugin (`1 + 1` shows 2). Route: `plugin/<folder name>`. -->
<script lang="ts">
  import { tick } from 'svelte';

  import IconButton from '../../components/IconButton.svelte';
  import type { PluginLogLevel } from '../../ipc/bindings/PluginLogLevel';
  import { pluginsIpc } from '../../ipc/plugins';
  import { toast } from '../../lib/toast.svelte';
  import { plugins } from './plugins.svelte';

  let { route }: { route: string; popout?: boolean } = $props();

  interface Entry {
    ts: number;
    kind: 'log' | 'in' | 'out' | 'err';
    level?: PluginLogLevel;
    text: string;
  }

  const MAX = 1000;
  const id = $derived(route.split('/')[1] ?? '');
  const info = $derived(plugins.view?.list.find((p) => p.id === id));
  const running = $derived(info?.status === 'running');

  let entries = $state<Entry[]>([]);
  let code = $state('');
  let history: string[] = [];
  let recall = -1;
  let list = $state<HTMLElement>();
  let busy = $state(false);

  function add(e: Entry) {
    const stick = !list || list.scrollHeight - list.scrollTop - list.clientHeight < 40;
    entries = [...entries.slice(-(MAX - 1)), e];
    if (stick) void tick().then(() => list?.scrollTo({ top: list.scrollHeight }));
  }

  $effect(() => {
    const plugin = id;
    if (!plugin) return;
    pluginsIpc
      .logs(plugin)
      .then((lines) => {
        entries = lines.map((l) => ({ ts: l.ts, kind: 'log', level: l.level, text: l.text }));
        void tick().then(() => list?.scrollTo({ top: list.scrollHeight }));
      })
      .catch(() => {});
    const un = pluginsIpc.onLog((from, l) => {
      if (from === plugin) add({ ts: l.ts, kind: 'log', level: l.level, text: l.text });
    });
    return () => void un.then((f) => f());
  });

  async function run() {
    const line = code.trim();
    if (!line || busy) return;
    history = [line, ...history.filter((h) => h !== line)].slice(0, 50);
    recall = -1;
    code = '';
    add({ ts: Date.now(), kind: 'in', text: line });
    busy = true;
    try {
      const out = await pluginsIpc.repl(id, line);
      if (out) add({ ts: Date.now(), kind: 'out', text: out });
    } catch (e) {
      add({ ts: Date.now(), kind: 'err', text: String(e) });
    } finally {
      busy = false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      void run();
    } else if ((e.key === 'ArrowUp' || e.key === 'ArrowDown') && history.length) {
      e.preventDefault();
      recall = Math.max(-1, Math.min(history.length - 1, recall + (e.key === 'ArrowUp' ? 1 : -1)));
      code = recall < 0 ? '' : history[recall];
    }
  }

  function clear() {
    entries = [];
    void pluginsIpc.clearLog(id);
  }

  function reload() {
    pluginsIpc.reload(id).catch((e) => toast.error(String(e)));
  }

  const time = (ts: number) => new Date(ts).toLocaleTimeString('en-GB', { hour12: false });
</script>

<div class="console">
  <div class="bar">
    <span class="name">{info?.info?.name ?? id}</span>
    <span class="state">{running ? 'Running' : 'Not running'}</span>
    <IconButton icon="refresh" label="Reload" hint="Reads its files again and starts it fresh" size={15} onclick={reload} />
    <IconButton icon="trash" label="Clear" size={15} onclick={clear} />
  </div>
  <div class="log" bind:this={list} role="log">
    {#each entries as e, i (i)}
      <div class="entry {e.kind} {e.level ?? ''}">
        <span class="ts">{time(e.ts)}</span>
        <span class="text">{e.kind === 'in' ? `> ${e.text}` : e.text}</span>
      </div>
    {/each}
  </div>
  <input
    class="field input"
    placeholder={running ? 'Lua, e.g. cyex.current_account()' : 'Turn the plugin on to run Lua in it'}
    aria-label="Lua to run"
    spellcheck="false"
    disabled={!running}
    bind:value={code}
    {onkeydown}
  />
</div>

<style>
  .console {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--page);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px 6px 12px;
    border-bottom: 1px solid var(--border);
  }
  .name {
    font-weight: 600;
  }
  .state {
    flex: 1;
    color: var(--dim);
    font-size: 0.8rem;
  }
  .log {
    flex: 1;
    overflow-y: auto;
    padding: 6px 0;
    font: 0.8rem/1.45 var(--font-mono);
  }
  .entry {
    display: flex;
    gap: 10px;
    padding: 1px 12px;
  }
  .ts {
    flex: none;
    color: var(--dim);
  }
  .text {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }
  .Debug .text {
    color: var(--muted);
  }
  .Warning .text {
    color: var(--yellow);
  }
  .Critical .text,
  .err .text {
    color: var(--red);
  }
  .in .text {
    color: var(--accent-text);
  }
  .input {
    margin: 8px;
    font-family: var(--font-mono);
  }
</style>
