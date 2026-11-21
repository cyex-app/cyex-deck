// V112-23 cyex.tv is "coming soon". Mirrors `cyex_core::CYEXTV_READY` (crates/core/src/lib.rs):
// while false its settings, Accounts row, picker side and emote-menu entry show disabled with
// "Coming soon", and its saved settings count as off. Flip both to `true` to bring everything back
// unchanged.

export const CYEXTV_READY: boolean = false;

/** The tag on what waits for cyex.tv. */
export const SOON = 'Coming soon';

/** A cyex.tv setting as it acts: the saved value only while cyex.tv is ready. */
export const cyextvOn = (saved: boolean, ready: boolean = CYEXTV_READY): boolean => ready && saved;
