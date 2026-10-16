// Fonts-fixed (user, 2026/10/16) The only fonts there are. Same list, same order as `FONTS`
// in crates/settings/src/schema/general.rs (a test there checks this file names each).
// The first six come with the app (fonts.css), Segoe UI and Arial are always on Windows.
// IBM Plex Sans is the app font: the setting stores it as '' (see `fontValue`).

export const FONTS = ['Inter', 'Roboto', 'Open Sans', 'Lato', 'JetBrains Mono', 'IBM Plex Sans', 'Segoe UI', 'Arial'] as const;

export const APP_FONT = 'IBM Plex Sans';

/** What the setting holds for a pick: the app font is empty. */
export const fontValue = (name: string): string => (name === APP_FONT ? '' : name);

/** The list name a saved value means ('' is the app font). */
export const fontName = (value: string): string => (value === '' ? APP_FONT : value);
