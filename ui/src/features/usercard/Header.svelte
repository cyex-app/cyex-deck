<!-- V101-05 The usercard's head, after the 7TV extension's: banner behind, picture, name in
     its color or 7TV paint, the paint's name, roles and every badge they own (V102-13), and
     small buttons for the note, the logs and the Twitch profile. V101-23 switches the 7TV
     picture for Twitch's. V102-25 A flag when Twitch hides the account. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Badge } from '../../ipc/bindings/Badge';
  import type { Profile } from '../../ipc/bindings/Profile';
  import type { NameColor } from '../../ipc/bindings/NameColor';
  import { systemIpc } from '../../ipc/system';
  import { settings } from '../../lib/settings.svelte';
  import { streamer } from '../../lib/streamer.svelte';
  import EarnedBadges from './EarnedBadges.svelte';
  import { suspendedFlag } from './suspended';
  import { avatars } from '../emotes/avatars.svelte';
  import { looks } from '../emotes/looks.svelte';
  import { toggleWatch, watched } from './watch';
  import { copy } from '../../lib/clipboard';

  interface Props {
    profile: Profile;
    channel: string;
    /** From their last message here, when there is one. */
    badges: Badge[];
    color: NameColor | null;
    /** The card is the channel's own (no profile link needed). */
    isChannel: boolean;
    logsUrl: string;
    editing: boolean;
  }

  let { profile, channel, badges, color, isChannel, logsUrl, editing = $bindable() }: Props = $props();

  // V101-23 Per card: every card starts on the 7TV picture.
  let twitchPic = $state(false);

  const note = $derived(settings.v.chat.users[profile.id]);
  const localized = $derived(profile.displayName.toLowerCase() !== profile.login);
  const seventv = $derived(avatars.of(profile.login));
  const picture = $derived(streamer.hides('hideAvatars') ? '' : (!twitchPic && seventv) || profile.avatar || '');
  const paintId = $derived(settings.v.chat.paints.seventv ? (looks.look(profile.id)?.paint ?? null) : null);
  const paint = $derived(looks.paintClass(paintId));
  const paintName = $derived(paint ? looks.paintName(paintId) : null);
  const colorVars = $derived(color ? `--cd:${color.dark};--cl:${color.light}` : '');
  const bell = $derived(watched(settings.v.highlights.users, profile.login));
  const flag = $derived(suspendedFlag(profile.suspended));

  function flipBell() {
    void settings.patch('highlights', { users: toggleWatch(settings.v.highlights.users, profile.login) });
  }

  function copyId() {
    void copy(profile.id, 'User ID');
  }
</script>

<header class:banner={!!profile.banner} style:--banner={profile.banner ? `url("${profile.banner}")` : null}>
  <div class="pic">
    <Avatar src={picture} name={profile.displayName} size={60} />
    {#if seventv && profile.avatar && !streamer.hides('hideAvatars')}
      <!-- V102-12 The logo of the picture shown; a click shows the other one. -->
      <button
        class="which"
        aria-label={twitchPic ? 'Twitch picture' : '7TV picture'}
        onclick={() => (twitchPic = !twitchPic)}
        use:tooltip={twitchPic ? 'Show the 7TV picture' : 'Show the Twitch picture'}
        ><Icon name={twitchPic ? 'twitch' : 'seventv'} size={11} /></button
      >
    {/if}
  </div>
  <div class="who">
    <div class="name" style={colorVars}>
      <strong class={paint ?? ''} class:painted={!!paint} class:colored={!!color}>{note?.nickname || profile.displayName}</strong>
      {#if localized || note?.nickname}<span class="login">{profile.login}</span>{/if}
    </div>
    {#if paintName}<span class="paint" style={colorVars}>{paintName}</span>{/if}
    <div class="marks">
      {#if flag}<span class="role suspended {flag.tone}" use:tooltip={flag.hint}>{flag.label}</span>{/if}
      {#if profile.staff}<span class="role staff">Staff</span>{/if}
      {#if profile.partner}<span class="role partner">Partner</span>{/if}
      {#if profile.affiliate}<span class="role affiliate">Affiliate</span>{/if}
    </div>
    <EarnedBadges login={profile.login} userId={profile.id} {channel} fallback={badges} />
    <button class="id" onclick={copyId} use:tooltip={'Copy user ID'}>ID {profile.id}</button>
  </div>
  <div class="tools">
    <IconButton
      icon="pencil"
      label="Nickname and note"
      hint="Only you see them."
      size={14}
      active={editing}
      toggle
      onclick={() => (editing = !editing)}
    />
    <IconButton
      icon="bell"
      label={bell ? 'Stop highlighting' : 'Highlight their messages'}
      hint="In every chat. Color and sound in Settings → Highlights."
      size={14}
      active={bell}
      toggle
      onclick={flipBell}
    />
    {#if logsUrl}
      <IconButton icon="scroll" label="All logs" hint="logs.cyex.app" size={14} onclick={() => systemIpc.openLink(logsUrl)} />
    {/if}
    {#if !isChannel}
      <IconButton
        icon="popout"
        label="Twitch profile"
        size={14}
        onclick={() => systemIpc.openLink(`https://www.twitch.tv/${profile.login}`)}
      />
    {/if}
  </div>
</header>

<style>
  header {
    position: relative;
    display: flex;
    gap: 12px;
    padding: 12px 14px 10px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  /* The banner stays readable under the name in both themes. */
  header.banner {
    background:
      linear-gradient(color-mix(in srgb, var(--surface) 72%, transparent), var(--surface)),
      var(--banner) center top / cover;
  }
  /* V102-09 The picture is square (Avatar's small corner radius). */
  .pic {
    position: relative;
    flex: none;
    align-self: flex-start;
  }
  .which {
    position: absolute;
    right: -6px;
    bottom: -4px;
    display: grid;
    place-items: center;
    width: 19px;
    height: 17px;
    padding: 0;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
  }
  .which:hover {
    color: var(--text);
  }
  .who {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  /* Clear of the buttons in the corner. */
  .name {
    display: flex;
    gap: 6px;
    align-items: baseline;
    min-width: 0;
    padding-right: 90px;
    font-size: 1.1rem;
  }
  .name strong {
    font-weight: 800;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .name .colored {
    color: var(--cd);
  }
  :global(:root[data-theme='light']) .name .colored {
    color: var(--cl);
  }
  .login {
    color: var(--dim);
    font-size: 0.8rem;
  }
  .paint {
    align-self: flex-start;
    padding: 0 6px;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--cd, var(--accent)) 22%, transparent);
    color: var(--text);
    font-size: 0.7rem;
    font-weight: 700;
  }
  .marks {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    align-items: center;
  }
  .marks:empty {
    display: none;
  }
  .role {
    padding: 0 6px;
    border-radius: var(--radius);
    font-size: 0.7rem;
    font-weight: 700;
  }
  .staff {
    background: color-mix(in srgb, var(--red) 20%, transparent);
    color: var(--red);
  }
  .partner {
    background: color-mix(in srgb, var(--twitch) 20%, transparent);
    color: var(--twitch);
  }
  .affiliate {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  /* V102-25 */
  .suspended.red {
    background: color-mix(in srgb, var(--red) 20%, transparent);
    color: var(--red);
  }
  .suspended.yellow {
    background: color-mix(in srgb, var(--yellow) 20%, transparent);
    color: var(--yellow);
  }
  .suspended.grey {
    background: var(--surface-hover);
    color: var(--muted);
  }
  .id {
    align-self: flex-start;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
    font: 0.72rem var(--font-mono);
  }
  .id:hover {
    color: var(--text);
  }
  .tools {
    position: absolute;
    top: 6px;
    right: 6px;
    display: flex;
    gap: 2px;
  }
</style>
