<!-- V109-C04 Creator dashboard: a pop-up like Settings, modelled on Twitch's creator dashboard.
     Sections on the left, the open one on the right; the last one opened opens next time. It
     acts on an account's own channel: the default, or the one picked in the header. Clips is
     the clips manager (V109-C03); Videos, Followers, Channel points and Moderation moved here
     from the stream manager; Roles Manager is in both (one component). All need a GQL sign-in. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import GateFlag from '../../components/GateFlag.svelte';
  import Icon from '../../components/Icon.svelte';
  import PanelBox from '../../components/PanelBox.svelte';
  import Select from '../../components/Select.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { accounts } from '../../lib/accounts.svelte';
  import ClipsManager from '../clips/ClipsManager.svelte';
  import { accountChoices } from '../cosmetics/accountChoices';
  import FollowersCard from '../streammanager/FollowersCard.svelte';
  import ModSettingsCard from '../streammanager/ModSettingsCard.svelte';
  import { setOwner } from '../streammanager/owner';
  import RewardsCard from '../streammanager/RewardsCard.svelte';
  import RolesManagerCard from '../streammanager/RolesManagerCard.svelte';
  import VideosCard from '../streammanager/VideosCard.svelte';
  import { GROUPS, keepSection, lastSection, SECTIONS, type SectionId } from './sections';

  let section = $state<SectionId>(lastSection());
  const open = $derived(SECTIONS.find((s) => s.id === section) ?? SECTIONS[0]);

  function go(id: SectionId) {
    section = id;
    keepSection(id);
  }

  let chosen = $state<string | null>(null);
  const picked = $derived(
    accounts.list.find((a) => a.userId === chosen) ?? accounts.current ?? accounts.list[0] ?? null,
  );
  const choices = $derived(accountChoices(accounts.list));
  const gql = $derived(!!picked && accounts.hasGql(picked.userId));

  // The panels act on the picked account's channel.
  setOwner({
    get who() {
      return picked
        ? { accountId: picked.userId, login: picked.login, display: picked.displayName || picked.login, avatar: picked.avatar }
        : null;
    },
    get account() {
      return picked?.userId ?? null;
    },
  });

  let nav = $state<HTMLElement>();
</script>

<div class="dash">
  <header>
    <h1>Creator dashboard</h1>
    {#if accounts.list.length > 1 && picked}
      <Select label="Account" width="220px" value={picked.userId} options={choices} onchange={(id) => (chosen = id)} />
    {:else if picked}
      <span class="one"><Avatar src={picked.avatar} name={picked.displayName} size={20} />{picked.displayName}</span>
    {/if}
  </header>
  <div class="split">
    <nav bind:this={nav} aria-label="Sections">
      {#each GROUPS as g (g.label)}
        <div class="group">{g.label}</div>
        {#each g.sections as s (s.id)}
          <button
            class:on={section === s.id}
            aria-label={s.label}
            aria-current={section === s.id ? 'page' : undefined}
            onclick={() => go(s.id)}
            use:tooltip={{ label: s.label, when: () => (nav?.offsetWidth ?? 999) < 100 }}
          >
            <Icon name={s.icon} size={15} />
            <span>{s.label}</span>
          </button>
        {/each}
      {/each}
    </nav>
    <div class="content">
      <div class="head">
        <h2>{open.label}</h2>
        <p>{open.hint}</p>
      </div>
      {#if !picked}
        <p class="note">Sign in with Twitch for your channel to use this.</p>
      {:else if !gql}
        <p class="note"><GateFlag kind="gql" /><span>This needs the sign-in with a code.</span></p>
      {:else}
        <div class="panel">
          <!-- A new account starts each panel fresh. -->
          {#key `${picked.userId}/${section}`}
            {#if section === 'clips'}
              <ClipsManager account={picked.userId} />
            {:else}
              <PanelBox ctl={{ unlocked: false, bare: true }}>
                {#if section === 'videos'}<VideosCard />
                {:else if section === 'followers'}<FollowersCard />
                {:else if section === 'roles'}<RolesManagerCard />
                {:else if section === 'rewards'}<RewardsCard />
                {:else if section === 'modset'}<ModSettingsCard />
                {/if}
              </PanelBox>
            {/if}
          {/key}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .dash {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    /* Clear of the sheet's close button. */
    padding: 12px 48px 12px 20px;
    border-bottom: 1px solid var(--border);
  }
  h1 {
    margin: 0 4px 0 0;
    font-size: 1.05rem;
  }
  .one {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .split {
    flex: 1;
    display: flex;
    min-height: 0;
    container-type: inline-size;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 200px;
    flex: none;
    padding: 4px 10px 14px;
    overflow-y: auto;
    border-right: 1px solid var(--border);
  }
  .group {
    padding: 12px 10px 4px;
    color: var(--dim);
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  nav button {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: none;
    height: 30px;
    padding: 0 10px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    font-weight: 600;
    text-align: left;
    white-space: nowrap;
  }
  nav button:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  nav button.on {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  /* V109-F84 Forced colours drop the tint: the current page gets a Highlight outline. */
  @media (forced-colors: active) {
    nav button.on {
      outline: 2px solid Highlight;
      outline-offset: -2px;
    }
  }
  .content {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    padding: 16px 20px 20px;
    gap: 12px;
  }
  .head h2 {
    margin: 0;
    font-size: 0.95rem;
  }
  .head p {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  .note {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    color: var(--muted);
    font-size: 0.9rem;
  }
  /* Narrow: icons only, like Settings. */
  @container (max-width: 600px) {
    nav {
      width: 52px;
      padding-inline: 8px;
    }
    nav button span {
      display: none;
    }
    .group {
      height: 1px;
      margin: 8px 4px;
      padding: 0;
      overflow: hidden;
      background: var(--border);
      color: transparent;
    }
    .content {
      padding: 12px;
    }
  }
</style>
