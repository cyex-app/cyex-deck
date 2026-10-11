<!-- 1.49 The usercard's mod bar, shown when your account moderates this channel (1.57), in
     the 7TV extension's order (V101-05): ban or unban, warn, your timeout lengths (2.11),
     and the rest behind ⋯. Says when they're banned or timed out right now. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { openMenu } from '../../components/menu.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { BanStatus } from '../../ipc/bindings/BanStatus';
  import type { Profile } from '../../ipc/bindings/Profile';
  import { modIpc } from '../../ipc/moderation';
  import { formatDate, formatTime } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { mods, timeouts, treatItems } from '../modview/moderation.svelte';
  import type { Standing } from '../modview/standing';

  interface Props {
    profile: Profile;
    channel: string;
    account: string;
    /** From the mod logs; `undefined` while unknown (no GQL sign-in). */
    ban: BanStatus | null | undefined;
    /** After an action, so the card reads the logs again. */
    onchange: () => void;
    /** M12 Mod or VIP, from their last line's badges. */
    is?: Standing;
  }

  let { profile, channel, account, ban, onchange, is }: Props = $props();

  const role = $derived(mods.role(account, channel));
  let busy = $state(false);
  let warning = $state(false);
  let reason = $state('');

  async function run(name: string, args: string) {
    busy = true;
    try {
      const line = await modIpc.run(account, channel, name, args);
      if (line) toast.info(line);
      onchange();
    } catch (e) {
      toast.error(String(e));
    } finally {
      busy = false;
    }
  }

  async function warn() {
    const why = reason.trim();
    if (!why) return;
    await run('warn', `${profile.login} ${why}`);
    reason = '';
    warning = false;
  }

  function more(e: MouseEvent) {
    if (!role) return;
    const line = (text: string) => {
      const [name, ...rest] = text.slice(1).split(' ');
      void run(name, rest.join(' '));
    };
    openMenu(
      e.currentTarget as HTMLElement,
      [...(ban ? [] : [{ label: 'Unban', icon: 'undo' as const, run: () => line(`/unban ${profile.login}`) }]), ...treatItems(role, profile.login, line, is)],
      'end',
    );
  }

  const until = $derived.by(() => {
    if (!ban?.until) return '';
    const d = new Date(ban.until);
    const today = formatDate(d) === formatDate(new Date());
    return today ? formatTime(d, settings.v.chat.timestamps) : formatDate(d);
  });
</script>

{#if role}
  <section>
    <div class="bar">
      {#if ban}
        <button class="act ok" disabled={busy} onclick={() => run('unban', profile.login)} use:tooltip={'Unban'}>
          <Icon name="undo" size={15} />
        </button>
      {:else}
        <button class="act bad" disabled={busy} onclick={() => run('ban', profile.login)} use:tooltip={'Ban'}>
          <Icon name="gavel" size={15} />
        </button>
      {/if}
      <button
        class="act warn"
        class:on={warning}
        disabled={busy}
        onclick={() => (warning = !warning)}
        use:tooltip={{ label: 'Warn', hint: 'They have to read it before they can chat again.' }}
      >
        <Icon name="alert" size={15} />
      </button>
      <div class="times">
        {#each timeouts() as t (t.secs)}
          <button disabled={busy} onclick={() => run('timeout', `${profile.login} ${t.secs}`)} use:tooltip={`${t.label} timeout`}
            >{t.label}</button
          >
        {/each}
      </div>
      <IconButton icon="more" label="More" hint="Unban, monitor, restrict, mod, VIP." size={15} onclick={more} />
    </div>
    {#if warning}
      <form
        class="reason"
        onsubmit={(e) => {
          e.preventDefault();
          void warn();
        }}
      >
        <!-- svelte-ignore a11y_autofocus -->
        <input class="field" maxlength="500" placeholder="Why? They'll see this." bind:value={reason} autofocus />
        <button class="send" disabled={busy || !reason.trim()}>Warn</button>
      </form>
    {/if}
    {#if ban}
      <p class="state">
        {ban.until ? `Timed out until ${until}` : 'Banned'}{ban.by ? ` by ${ban.by}` : ''}{ban.reason ? `: ${ban.reason}` : ''}
      </p>
    {/if}
  </section>
{/if}

<style>
  section {
    border-top: 1px solid var(--border);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 4px 8px;
  }
  .act {
    display: grid;
    place-items: center;
    flex: none;
    width: 28px;
    height: 26px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
  }
  .act:hover,
  .act.on {
    background: var(--row-hover);
  }
  .bad:hover {
    color: var(--red);
  }
  .ok:hover {
    color: var(--green);
  }
  .warn:hover,
  .warn.on {
    color: var(--yellow);
  }
  .times {
    display: flex;
    flex: 1;
    flex-wrap: wrap;
    gap: 2px;
    min-width: 0;
  }
  .times button {
    flex: 1;
    min-width: 30px;
    height: 24px;
    padding: 0 4px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 600;
  }
  .times button:hover {
    background: var(--row-hover);
    color: var(--text);
  }
  .reason {
    display: flex;
    gap: 6px;
    padding: 0 8px 6px;
  }
  .reason .field {
    flex: 1;
    min-width: 0;
  }
  .send {
    padding: 0 10px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.8rem;
  }
  .state {
    margin: 0;
    padding: 0 14px 6px;
    color: var(--red);
    font-size: 0.8rem;
  }
</style>
