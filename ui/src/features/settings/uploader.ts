import type { Uploader } from '../../ipc/bindings/Uploader';

/** V106-14 The uploader fields in form order. Headers and Delete link are optional (round 2, O1). */
export const UPLOADER_FIELDS = [
  ['url', 'Upload URL', true],
  ['formField', 'Form field', true],
  ['headers', 'Headers', false],
  ['link', 'Image link', true],
  ['deletionLink', 'Delete link', false],
] as const;

const REQUIRED = UPLOADER_FIELDS.filter((f) => f[2]).map((f) => f[0]);
type Required = (typeof REQUIRED)[number];

/** The required fields left empty, in form order. Empty = blank after trim. */
export function uploaderMissingKeys(u: Uploader): Required[] {
  return REQUIRED.filter((k) => u[k].trim() === '');
}
