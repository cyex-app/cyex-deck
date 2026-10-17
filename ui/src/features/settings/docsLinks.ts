// V106-12 Where a settings page's "Docs" link goes on cyex.app/deck/docs. Anchors are the
// docs page's section ids; `s-<page id>` is a settings page's own block there.
export const DOCS_URL = 'https://cyex.app/deck/docs/';
export const DOCS = {
  commands: 'commands',
  automations: 'commands',
  plugins: 'plugins',
  filters: 's-filters',
  hotkeys: 'shortcuts',
  data: 'data',
} as const;
export type DocsAnchor = keyof typeof DOCS;
export const docsUrl = (a: DocsAnchor): string => `${DOCS_URL}#${DOCS[a]}`;
