<!-- V105-11 Image uploads: where pasted and dropped images in the chat box go (Chatterino's
     uploader). Stays on this PC; never in exports. -->
<script lang="ts">
  import IconButton from '../../../components/IconButton.svelte';
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { Uploader } from '../../../ipc/bindings/Uploader';
  import { settingsIpc } from '../../../ipc/settings';
  import { settings } from '../../../lib/settings.svelte';
  import ResetSection from '../ResetSection.svelte';

  const u = $derived(settings.v.uploader);
  const save = (patch: Partial<Uploader>) => void settings.patch('uploader', patch);

  let show = $state(false);
  // The headers live in the credential store; the settings value is always empty.
  let headers = $state('');
  const loadHeaders = () => void settingsIpc.uploaderHeaders().then((h) => (headers = h));
  $effect(loadHeaders);
  const saveHeaders = (value: string) => {
    headers = value;
    save({ headers: value });
  };
</script>

<h2>Image uploads</h2>
<p class="intro">
  Images you paste or drop into the chat box are uploaded here. These settings stay on this PC and never go into
  exports. Headers are stored securely on this PC.
</p>

<Row setting="uploader.enabled" label="Upload images you paste or drop">
  <Toggle label="Upload images you paste or drop" checked={u.enabled} onchange={(v) => save({ enabled: v })} />
</Row>

<Row
  setting="uploader.confirm"
  label="Ask before uploading"
  hint="One question per paste or drop."
>
  <Toggle label="Ask before uploading" checked={u.confirm} onchange={(v) => save({ confirm: v })} />
</Row>

<Row setting="uploader.url" label="Upload URL" stacked>
  <input
    class="field wide"
    spellcheck="false"
    autocomplete="off"
    aria-label="Upload URL"
    value={u.url}
    onchange={(e) => save({ url: e.currentTarget.value })}
  />
</Row>

<Row setting="uploader.formField" label="Form field" stacked>
  <input
    class="field wide"
    spellcheck="false"
    autocomplete="off"
    aria-label="Form field"
    placeholder="e.g. file"
    value={u.formField}
    onchange={(e) => save({ formField: e.currentTarget.value })}
  />
</Row>

<Row setting="uploader.headers" label="Headers" stacked>
  <div class="secret">
    <input
      class="field wide"
      type={show ? 'text' : 'password'}
      spellcheck="false"
      autocomplete="off"
      aria-label="Headers"
      placeholder="e.g. Authorization: key; X-Other: value"
      value={headers}
      onchange={(e) => saveHeaders(e.currentTarget.value)}
    />
    <IconButton icon={show ? 'eyeOff' : 'eye'} label={show ? 'Hide' : 'Show'} onclick={() => (show = !show)} />
  </div>
</Row>

<Row
  setting="uploader.link"
  label="Image link"
  hint="Where the link is in the answer, like {'{data.link}'}. Empty: the whole answer."
  stacked
>
  <input
    class="field wide"
    spellcheck="false"
    autocomplete="off"
    aria-label="Image link"
    placeholder={'e.g. {url}'}
    value={u.link}
    onchange={(e) => save({ link: e.currentTarget.value })}
  />
</Row>

<Row setting="uploader.deletionLink" label="Delete link" stacked>
  <input
    class="field wide"
    spellcheck="false"
    autocomplete="off"
    aria-label="Delete link"
    placeholder={'e.g. {delete}'}
    value={u.deletionLink}
    onchange={(e) => save({ deletionLink: e.currentTarget.value })}
  />
</Row>

<!-- The reset clears the stored headers too: read them again. -->
<ResetSection partition="uploader" onreset={loadHeaders} />

<style>
  .intro {
    margin: -6px 0 6px;
    color: var(--dim);
  }
  .wide {
    width: 100%;
    min-width: 0;
  }
  .secret {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .secret .field {
    flex: 1;
  }
</style>
