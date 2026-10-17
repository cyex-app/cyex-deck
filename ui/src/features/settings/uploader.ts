import type { Uploader } from '../../ipc/bindings/Uploader';

/** V106-14 The uploader fields in form order. Headers and Delete link are optional (round 2, O1). */
export const UPLOADER_FIELDS = [
  ['url', 'Upload URL', true],
  ['formField', 'Form field', true],
  ['headers', 'Headers', false],
  ['link', 'Image link', true],
  ['deletionLink', 'Delete link', false],
] as const;

type Required = 'url' | 'formField' | 'link';

/** The required fields left empty, in form order. Empty = blank after trim. */
export function uploaderMissingKeys(u: Uploader): Required[] {
  const keys: Required[] = ['url', 'formField', 'link'];
  return keys.filter((k) => u[k].trim() === '');
}

/** The labels of the required fields left empty. */
export function uploaderMissing(u: Uploader): string[] {
  const label = (k: string) => UPLOADER_FIELDS.find(([key]) => key === k)?.[1] ?? k;
  return uploaderMissingKeys(u).map(label);
}
