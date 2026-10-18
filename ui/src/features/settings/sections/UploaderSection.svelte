<!-- V105-11 Image uploads: where pasted and dropped images in the chat box go (Chatterino's
     uploader). Stays on this PC; never in exports. V106-14: grouped fields, three required. -->
<script lang="ts">
  import IconButton from '../../../components/IconButton.svelte';
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { Uploader } from '../../../ipc/bindings/Uploader';
  import { settingsIpc } from '../../../ipc/settings';
  import { settings } from '../../../lib/settings.svelte';
  import FieldGroup from '../FieldGroup.svelte';
  import FormField from '../FormField.svelte';
  import ResetSection from '../ResetSection.svelte';
  import SectionHead from '../SectionHead.svelte';
  import { uploaderMissingKeys } from '../uploader';

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

  const gone = $derived(uploaderMissingKeys(u));
  let tried = $state(false);
  const loud = $derived(u.enabled || tried);
  const problem = (key: 'url' | 'formField' | 'link') => (loud && gone.includes(key) ? 'Fill this in.' : null);

  /** Turning on needs the required fields; turning off always works. s3-13 A required field
   *  cleared while on keeps it on: `loud` marks the field and the switch's hint says why. */
  function toggle(on: boolean) {
    if (on && gone.length) {
      tried = true;
      document.querySelector<HTMLElement>(`[data-setting="uploader.${gone[0]}"] input`)?.focus();
      return;
    }
    save({ enabled: on });
  }
</script>

<SectionHead title="Image uploads" level={2}>
  Images you paste or drop into the chat box go to your upload host. These settings stay on this PC and never go
  into exports.
</SectionHead>

<Row
  setting="uploader.enabled"
  label="Upload images you paste or drop"
  hint={gone.length ? (u.enabled ? 'Won’t upload until the required fields below are filled in.' : 'Fill in every required field below first.') : undefined}
>
  <Toggle label="Upload images you paste or drop" checked={u.enabled} onchange={toggle} />
</Row>

<Row
  setting="uploader.confirm"
  label="Ask before uploading"
  hint="One question per paste or drop."
>
  <Toggle label="Ask before uploading" checked={u.confirm} onchange={(v) => save({ confirm: v })} />
</Row>

<FieldGroup legend="Upload host" hint="Copy these from your host’s ShareX or Chatterino settings.">
  <FormField setting="uploader.url" label="Upload URL" required hint="Where images are sent." problem={problem('url')}>
    <input
      class="field"
      spellcheck="false"
      autocomplete="off"
      value={u.url}
      onchange={(e) => save({ url: e.currentTarget.value })}
    />
  </FormField>

  <FormField
    setting="uploader.formField"
    label="Form field"
    required
    hint="The name the image goes under, often “file”."
    problem={problem('formField')}
  >
    <input
      class="field"
      spellcheck="false"
      autocomplete="off"
      value={u.formField}
      onchange={(e) => save({ formField: e.currentTarget.value })}
    />
  </FormField>

  <FormField
    setting="uploader.headers"
    label="Headers"
    hint="Sent with every upload, like a key. Kept securely on this PC. Empty: none are sent."
  >
    <div class="secret">
      <input
        class="field"
        type={show ? 'text' : 'password'}
        spellcheck="false"
        autocomplete="off"
        value={headers}
        onchange={(e) => saveHeaders(e.currentTarget.value)}
      />
      <IconButton icon={show ? 'eyeOff' : 'eye'} label={show ? 'Hide' : 'Show'} onclick={() => (show = !show)} />
    </div>
  </FormField>

  <FormField
    setting="uploader.link"
    label="Image link"
    required
    hint={'Where the link is in the answer, like {data.link}.'}
    problem={problem('link')}
  >
    <input
      class="field"
      spellcheck="false"
      autocomplete="off"
      value={u.link}
      onchange={(e) => save({ link: e.currentTarget.value })}
    />
  </FormField>

  <FormField
    setting="uploader.deletionLink"
    label="Delete link"
    hint={'Where the delete link is in the answer, like {data.deletion}. Empty: no delete link is kept.'}
  >
    <input
      class="field"
      spellcheck="false"
      autocomplete="off"
      value={u.deletionLink}
      onchange={(e) => save({ deletionLink: e.currentTarget.value })}
    />
  </FormField>
</FieldGroup>

<!-- The reset clears the stored headers too: read them again. -->
<ResetSection partition="uploader" onreset={loadHeaders} />

<style>
  .secret {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .secret .field {
    flex: 1;
    min-width: 0;
  }
</style>
