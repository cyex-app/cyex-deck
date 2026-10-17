<!-- 1.14 Channel field: a name, user id or twitch.tv link, searched as you type. Suggestions come
     from Twitch's search plus the exact match (offline too), best match first (`matchChannels`).
     Nothing is picked until one result is left or you choose; Enter takes the first, arrows move. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Avatar from '../../components/Avatar.svelte';
  import Icon from '../../components/Icon.svelte';
  import Loading from '../../components/Loading.svelte';
  import type { ChannelInfo } from '../../ipc/bindings/ChannelInfo';
  import type { ChannelCard } from '../../ipc/bindings/ChannelCard';
  import { discoverIpc } from '../../ipc/browse';
  import { tabsIpc } from '../../ipc/tabs';
  import { formatNumber } from '../../lib/format';
  import { errorText } from '../../lib/errors';
  import { matchChannels } from './channelMatch';

  interface Props {
    text: string;
    picked: ChannelInfo | null;
    onsubmit: () => void;
  }

  let { text = $bindable(), picked = $bindable(), onsubmit }: Props = $props();

  const DEBOUNCE_MS = 300;

  let results = $state<ChannelInfo[]>([]);
  let status = $state<'idle' | 'looking' | 'done' | 'error'>('idle');
  let error = $state('');
  let asked = '';

  // Looks up what's typed; picking a result doesn't start a new lookup.
  $effect(() => {
    const input = text.trim();
    return untrack(() => {
      if (!input) {
        results = [];
        picked = null;
        status = 'idle';
        return;
      }
      // Already showing this channel, e.g. when editing.
      if (picked && [picked.login, picked.id].includes(input.toLowerCase())) return;
      picked = null;
      status = 'looking';
      const timer = setTimeout(() => void look(input), DEBOUNCE_MS);
      return () => clearTimeout(timer);
    });
  });

  /** Most suggestions listed. */
  const MAX = 6;

  function fromCard(c: ChannelCard): ChannelInfo {
    const s = c.stream;
    return {
      id: c.id,
      login: c.login,
      name: c.name,
      avatar: c.avatar,
      offlineImage: null,
      stream: s ? { rerun: s.rerun, title: s.title, game: s.game?.name ?? null, viewers: s.viewers, startedAt: s.startedAt } : null,
    };
  }

  async function look(input: string) {
    asked = input;
    try {
      let failed: unknown = null;
      const [exact, search] = await Promise.all([
        tabsIpc.lookup(input).catch((e) => ((failed = e), [] as ChannelInfo[])),
        input.length >= 2
          ? discoverIpc
              .search(input)
              .then((r) => r.channels.items.map(fromCard))
              .catch(() => [] as ChannelInfo[])
          : Promise.resolve([] as ChannelInfo[]),
      ]);
      if (asked !== input) return;
      const found = matchChannels(exact, search, input, MAX);
      if (!found.length && failed) throw failed;
      results = found;
      // One match left is the one, and so is an exact name (any case); the rest wait for a choice.
      const q = input.toLowerCase();
      picked = found.length === 1 ? found[0] : (found.find((c) => c.login === q) ?? null);
      status = 'done';
    } catch (e) {
      if (asked !== input) return;
      error = errorText(e);
      status = 'error';
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (picked) onsubmit();
      else if (results.length) picked = results[0];
    } else if ((e.key === 'ArrowDown' || e.key === 'ArrowUp') && results.length > 1) {
      e.preventDefault();
      const at = results.findIndex((c) => c.id === picked?.id);
      const step = e.key === 'ArrowDown' ? 1 : -1;
      picked = results[(at + step + results.length) % results.length];
    }
  }

  const shown = $derived(results.length ? results : picked ? [picked] : []);
</script>

<input
  class="field"
  placeholder="Channel name, user id or link"
  spellcheck="false"
  autocomplete="off"
  bind:value={text}
  {onkeydown}
/>

{#if status === 'looking' && !shown.length}
  <Loading />
{:else if status === 'error'}
  <p class="note bad">{error}</p>
{:else if status === 'done' && !results.length}
  <p class="note bad">No channel by that name or id.</p>
{:else if shown.length}
  {#if shown.length > 1}<p class="note">Pick one. Best match first.</p>{/if}
  <div class="results" role="radiogroup" aria-label="Channel">
    {#each shown as c (c.id)}
      <button
        class="result"
        class:on={picked?.id === c.id}
        role="radio"
        aria-checked={picked?.id === c.id}
        onclick={() => (picked = c)}
      >
        <Avatar src={c.avatar} name={c.name} size={32} />
        <span class="who">
          <span class="name">{c.name}{#if c.name.toLowerCase() !== c.login}<span class="login"> {c.login}</span>{/if}</span>
          <span class="sub">
            {#if c.stream}
              <span class="dot" class:rerun={c.stream.rerun}></span>
              {[c.stream.rerun ? 'Rerun' : 'Live', `${formatNumber(c.stream.viewers)} watching`, c.stream.game]
                .filter(Boolean)
                .join(' · ')}
            {:else}
              Offline · id {c.id}
            {/if}
          </span>
        </span>
        {#if picked?.id === c.id}<span class="check"><Icon name="check" size={15} /></span>{/if}
      </button>
    {/each}
  </div>
{/if}

<style>
  .field {
    width: 100%;
  }
  .note {
    margin: 8px 0 0;
    color: var(--muted);
    font-size: 0.82rem;
  }
  .bad {
    color: var(--yellow);
  }
  .results {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 8px;
  }
  .result {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    text-align: left;
  }
  .result:hover {
    border-color: var(--border-strong);
  }
  .result.on {
    border-color: var(--accent-border);
  }
  .who {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .name {
    font-weight: 700;
  }
  .login {
    color: var(--muted);
    font-weight: 500;
  }
  .sub {
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--muted);
    font-size: 0.78rem;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .dot {
    width: 7px;
    height: 7px;
    flex: none;
    border-radius: 50%;
    background: var(--live);
  }
  .dot.rerun {
    background: var(--yellow);
  }
  .check {
    display: flex;
    color: var(--accent-text);
  }
</style>
