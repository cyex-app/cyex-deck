<!-- 1.35 Edit the stream title and change the category. Categories are searched as you
     type; saving sends only what changed. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import type { CategoryHit } from '../../ipc/bindings/CategoryHit';
  import { smIpc } from '../../ipc/streammanager';
  import { toast } from '../../lib/toast.svelte';
  import Card from '../../components/PanelCard.svelte';

  const TITLE_MAX = 140;

  let saved = $state({ title: '', gameId: '', gameName: '' });
  let title = $state('');
  let game = $state<{ id: string; name: string }>({ id: '', name: '' });
  let query = $state('');
  let hits = $state<CategoryHit[]>([]);
  let loaded = $state(false);
  let saving = $state(false);

  function load() {
    smIpc
      .info()
      .then((i) => {
        saved = { title: i.title, gameId: i.gameId, gameName: i.gameName };
        title = i.title;
        game = { id: i.gameId, name: i.gameName };
        loaded = true;
      })
      .catch((e) => toast.error(String(e)));
  }
  load();

  // Search after a short pause in typing.
  $effect(() => {
    const q = query.trim();
    if (!q) {
      hits = [];
      return;
    }
    const t = setTimeout(() => {
      smIpc
        .categories(q)
        .then((h) => (hits = h))
        .catch(() => (hits = []));
    }, 250);
    return () => clearTimeout(t);
  });

  const changed = $derived(loaded && (title.trim() !== saved.title || game.id !== saved.gameId));

  function pick(h: CategoryHit) {
    game = { id: h.id, name: h.name };
    query = '';
    hits = [];
  }

  function save() {
    const t = title.trim();
    saving = true;
    smIpc
      .setInfo({
        title: t !== saved.title ? t : null,
        gameId: game.id !== saved.gameId ? game.id : null,
        gameName: game.id !== saved.gameId ? game.name : null,
        language: null,
        tags: null,
      })
      .then(() => {
        saved = { title: t, gameId: game.id, gameName: game.name };
        toast.info('Saved.');
      })
      .catch((e) => toast.error(String(e)))
      .finally(() => (saving = false));
  }

  const art = (url: string) => url.replace('{width}', '26').replace('{height}', '35');
</script>

<Card title="Stream info">
  <label class="label" for="sm-title">Title</label>
  <div class="count-wrap">
    <input id="sm-title" class="field" maxlength={TITLE_MAX} bind:value={title} disabled={!loaded} />
    <span class="count" class:near={title.length > TITLE_MAX - 15}>{title.length}/{TITLE_MAX}</span>
  </div>

  <span class="label">Category</span>
  <div class="line current">
    <span class="game">{game.name || 'None'}</span>
    {#if game.id}
      <button class="link" onclick={() => (game = { id: '', name: '' })}>Clear</button>
    {/if}
  </div>
  <input class="field" placeholder="Search categories" bind:value={query} disabled={!loaded} />
  {#if hits.length}
    <ul class="hits">
      {#each hits as h (h.id)}
        <li>
          <button onclick={() => pick(h)}>
            {#if h.boxArtUrl}<img src={art(h.boxArtUrl)} alt="" width="26" height="35" loading="lazy" />{/if}
            <span>{h.name}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <div class="line">
    <Button small variant="primary" disabled={!changed || saving || !title.trim()} onclick={save}>Save</Button>
    {#if changed}<button class="link" onclick={load}>Undo</button>{/if}
  </div>
</Card>

<style>
  .count-wrap {
    position: relative;
    display: flex;
  }
  .count-wrap .field {
    flex: 1;
    padding-right: 56px;
  }
  .count {
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 0.72rem;
    color: var(--dim);
    font-variant-numeric: tabular-nums;
  }
  .count.near {
    color: var(--yellow);
  }
  .current {
    font-size: 0.85rem;
  }
  .game {
    font-weight: 600;
  }
  .hits {
    margin: 0;
    padding: 0;
    list-style: none;
    max-height: 180px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .hits button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 4px 8px;
    background: none;
    border: 0;
    text-align: left;
    cursor: pointer;
  }
  .hits button:hover {
    background: var(--row-hover);
  }
  .hits img {
    border-radius: 2px;
    flex-shrink: 0;
  }
  .link {
    background: none;
    border: 0;
    padding: 0;
    color: var(--muted);
    font-size: 0.8rem;
    cursor: pointer;
  }
  .link:hover {
    color: var(--text);
  }
</style>
