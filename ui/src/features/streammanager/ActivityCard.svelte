<!-- 1.35 What's happening on your channel: follows, subs, gifts, cheers, raids, reward
     redemptions and watch streaks, and with the sign-in with a code power-ups, hype trains
     and goals (D3), newest first. The chips filter by kind and are saved. D4 Once the
     channel's alerts play in Twitch's alert box, each alert can be played again or skipped.
     V103-15 Mod actions have their own panel; chips and list are shared with it. -->
<script lang="ts">
  import FeedList from '../../components/FeedList.svelte';
  import FilterChips from '../../components/FilterChips.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Card from '../../components/PanelCard.svelte';
  import type { ActivityFilters } from '../../ipc/bindings/ActivityFilters';
  import type { SmActivity } from '../../ipc/bindings/SmActivity';
  import { smIpc } from '../../ipc/streammanager';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { ACTIVITY_KINDS, alertAction, visibleActivity } from './logic';
  import { sm } from './streammanager.svelte';
  import { errorText } from '../../lib/errors';

  const filters = $derived(settings.v.streammanager.activity);
  const rows = $derived(visibleActivity(sm.activity, filters).map((a) => ({ ...a, key: a.id })));
  const alertBox = $derived(settings.v.streammanager.alertBox);
  const chips = $derived(ACTIVITY_KINDS.map((k) => ({ key: k.key, label: k.label, on: filters[k.key] })));

  function flip(key: string) {
    const k = key as keyof ActivityFilters;
    void settings.patch('streammanager', { activity: { [k]: !filters[k] } });
  }

  function alert(r: SmActivity, replay: boolean) {
    if (!r.alert) return;
    smIpc
      .alert(r.alert, replay)
      .then(() => toast.info(replay ? 'Alert queued again.' : 'Alert skipped.'))
      .catch((e) => toast.error(errorText(e)));
  }
</script>

<Card title="Activity" flush>
  {#snippet tools()}
    <FilterChips {chips} onflip={flip} />
  {/snippet}
  <FeedList
    {rows}
    empty={sm.activity.length ? 'Nothing matches the filters.' : 'New follows, subs, cheers, raids, rewards and more show up here.'}
  >
    {#snippet row(r)}
      {@const action = alertBox ? alertAction(r.status) : null}
      <span class="line">
        <span class="what">{r.text}</span>
        {#if action === 'replay'}
          <IconButton icon="refresh" label="Play the alert again" size={13} onclick={() => alert(r, true)} />
        {:else if action === 'skip'}
          <IconButton icon="chevronRight" label="Skip the alert" size={13} onclick={() => alert(r, false)} />
        {/if}
      </span>
    {/snippet}
  </FeedList>
</Card>

<style>
  .line {
    display: flex;
    gap: 6px;
  }
  .what {
    flex: 1;
    min-width: 0;
  }
</style>
