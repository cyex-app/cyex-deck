<!-- V1051-02 What saying yes to 7TV means. Shown before any 7TV call: in the consent card and
     in the sign-in choice. -->
<script lang="ts">
  let { name }: { name: string } = $props();
</script>

<p>cyex deck will sign in to 7TV as {name}, read the paints and badges you own, and change the ones you wear when you pick one.</p>
<p>Your 7TV sign-in stays in Windows Credential Manager on this PC. It’s never in exports or backups.</p>
<p>You can take this back any time in Settings → Accounts.</p>

<style>
  p {
    margin: 0 0 10px;
    color: var(--muted);
    line-height: 1.5;
    text-wrap: pretty;
  }
</style>
