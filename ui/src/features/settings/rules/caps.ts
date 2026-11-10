/** V110-F101 How many rows each settings list keeps. Rust trims a longer list on save, so Add
 * stops at these numbers instead of adding a row that vanishes. They mirror
 * `crates/settings/src/schema/commands.rs` (MAX_LINES, MAX_AUTOMATIONS) and
 * `crates/settings/src/schema/highlights/mod.rs` (MAX_*); caps.test.ts fails when they drift. */
export const CAPS = {
  /** Commands, and the name menu, message menu, chat menu and usercard button lists (MAX_LINES). */
  commandLines: 500,
  /** MAX_AUTOMATIONS */
  automations: 100,
  /** MAX_FILTERS */
  filters: 50,
  /** MAX_WORDS */
  phrases: 300,
  /** MAX_GLOBAL */
  globalList: 100,
  /** MAX_RULES: people, badges, apps and ignores. */
  rules: 200,
  /** MAX_NEVER: Never highlight and Silent chats. */
  names: 500,
} as const;
