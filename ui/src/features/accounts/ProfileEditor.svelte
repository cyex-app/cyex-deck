<!-- 2.2 The account's Twitch profile: display name (Twitch only allows other capitals) and
     bio. Saved on Twitch. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import type { TwitchProfile } from '../../ipc/bindings/TwitchProfile';
  import { profileIpc } from '../../ipc/notify';
  import { toast } from '../../lib/toast.svelte';
  import { errorText } from '../../lib/errors';

  let { account }: { account: string } = $props();

  const BIO_MAX = 300;

  let profile = $state.raw<TwitchProfile | null>(null);
  let name = $state('');
  let bio = $state('');
  let error = $state('');
  let saving = $state(false);

  const changed = $derived(!!profile && (name.trim() !== profile.displayName || bio.trim() !== profile.bio.trim()));
  const nameOk = $derived(
    !!profile && (name.trim().toLowerCase() === profile.login || /^(zh|ja|ko)/i.test(profile.language)),
  );

  $effect(() => {
    profileIpc
      .get(account)
      .then((p) => {
        profile = p;
        name = p.displayName;
        bio = p.bio;
      })
      .catch((e) => (error = errorText(e)));
  });

  async function save() {
    if (!profile) return;
    saving = true;
    try {
      await profileIpc.save(account, name.trim(), bio.trim());
      profile = { ...profile, displayName: name.trim(), bio: bio.trim() };
      toast.info('Saved on Twitch.');
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      saving = false;
    }
  }
</script>

<div class="profile">
  {#if error}
    <p class="err">{error}</p>
  {:else if !profile}
    <p class="muted">Loading…</p>
  {:else}
    <label>
      <span>Display name</span>
      <input class="field" bind:value={name} maxlength="25" spellcheck="false" />
      <small class:bad={!nameOk}>{nameOk ? 'Only the capitals can change.' : `Has to be ${profile.login} with any capitals.`}</small>
    </label>
    <label>
      <span>Bio</span>
      <textarea class="field" bind:value={bio} rows="3" maxlength={BIO_MAX}></textarea>
      <small>{[...bio].length}/{BIO_MAX}</small>
    </label>
    <div class="row">
      <Button small variant="primary" disabled={!changed || !nameOk || saving} onclick={save}>Save on Twitch</Button>
    </div>
  {/if}
</div>

<style>
  .profile {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 10px 0 4px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  label span {
    font-size: 0.82rem;
    color: var(--muted);
  }
  textarea {
    resize: vertical;
    font: inherit;
  }
  small {
    color: var(--dim);
    font-size: 0.74rem;
  }
  small.bad,
  .err {
    color: var(--red);
  }
  .muted {
    color: var(--dim);
  }
  .row {
    display: flex;
    justify-content: flex-end;
  }
</style>
