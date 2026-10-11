// V103-13 One searchable setting. Each group's file lists its pages' entries; `index.ts`
// joins them. A settings key with no entry fails `registry.test.ts`.

import type { PageId } from './pages';

export interface Entry {
  /** The settings key it changes, `partition.field` as in the settings files
   * (`chat.timestamps.show`), or `page:thing` for what isn't kept there (a sign-in, a
   * button). The page marks the control with it: `setting` on a Row, else `data-setting`. */
  id: string;
  page: PageId;
  /** What it's called, as in the list (a bit fuller where the row leans on its heading). */
  label: string;
  /** The row's hint, or a few words on what it does. */
  description?: string;
  /** Other words people may type for it. */
  keywords?: string[];
  /** More settings keys the same control changes. */
  keys?: string[];
  /** Entry to jump to while this one isn't on screen, e.g. a row shown only once a switch
   * above it is on. */
  anchor?: string;
}
