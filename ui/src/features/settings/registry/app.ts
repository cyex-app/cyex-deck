// V103-13 Settings in the App group: General, Appearance, Windows, Hotkeys, Plugins,
// Backup and sharing.

import type { Entry } from './entry';

export const APP: Entry[] = [
  // General
  {
    id: 'general.chatFontFamily',
    page: 'general',
    label: 'Chat font',
    description: 'Fonts that come with the app, your font files, and fonts on this PC.',
    keywords: ['typeface', 'text', 'Inter', 'Roboto', 'Open Sans', 'Lato', 'JetBrains Mono', 'Segoe UI', 'Arial', 'installed'],
  },
  {
    id: 'general.fonts',
    page: 'general',
    label: 'Add a font file',
    description: 'Use your own font for chat.',
    keywords: ['custom font', 'ttf', 'otf', 'woff'],
  },
  { id: 'general.chatFontSize', page: 'general', label: 'Chat font size', keywords: ['text size', 'bigger', 'smaller', 'zoom'] },
  { id: 'general.pinnedSizeStep', page: 'general', label: 'Pinned message size', description: 'How much larger than chat text.', keywords: ['pinned', 'pin', 'text size', 'bigger'] },
  {
    id: 'general.chatPosition',
    page: 'general',
    label: 'Chat position',
    description: 'Which side of the player or a clip the chat sits on.',
    keywords: ['chat side', 'left', 'right', 'top', 'bottom', 'layout'],
  },
  {
    id: 'general.tooltips',
    page: 'general',
    label: 'Tooltips',
    description: 'Short explanations when you hover buttons.',
    keywords: ['hints', 'hover', 'help'],
  },
  {
    id: 'general.smoothScroll',
    page: 'general',
    label: 'Smooth scrolling',
    description: 'The mouse wheel glides instead of jumping, everywhere in the app.',
    keywords: ['scroll', 'scrolling', 'smooth', 'glide', 'wheel', 'mouse', 'animation', 'chatterino'],
  },
  {
    id: 'general.minimizeToTray',
    page: 'general',
    label: 'Minimize to tray',
    description: 'Minimizing hides cyex deck in the tray instead of the taskbar.',
    keywords: ['taskbar', 'system tray', 'background'],
  },
  {
    id: 'general.updates.check',
    page: 'general',
    label: 'Check for updates',
    description: 'At startup and every 12 hours.',
    keywords: ['update', 'version', 'new version', 'github', 'upgrade'],
  },
  {
    id: 'general.updates.prereleases',
    page: 'general',
    label: 'Include pre-releases',
    description: 'Test builds that come before a release.',
    keywords: ['update', 'version', 'beta', 'pre-release', 'prerelease', 'test build'],
  },
  {
    id: 'general:checkNow',
    page: 'general',
    label: 'Version',
    description: 'Look for a new version right away.',
    keywords: ['update', 'version', 'new version', 'github', 'check', 'check now'],
  },

  // Appearance
  {
    id: 'appearance.theme',
    page: 'appearance',
    label: 'Theme',
    description: 'Dark, light or one of your own.',
    keywords: ['dark mode', 'light mode', 'colors'],
    keys: ['appearance.themeId'],
  },
  {
    id: 'appearance.themes',
    page: 'appearance',
    label: 'Your own themes',
    description: 'Make your own with New, or paste a code someone shared.',
    keywords: ['custom theme', 'wallpaper', 'background picture', 'theme code', 'preset', 'colors', 'blur', 'dim'],
  },
  {
    id: 'appearance.accent',
    page: 'appearance',
    label: 'Accent color',
    description: 'The color of buttons, switches and links.',
    keywords: ['highlight color', 'blue'],
  },
  {
    id: 'appearance.tabDensity',
    page: 'appearance',
    label: 'Tab size',
    description: 'Compact fits more tabs.',
    keywords: ['density', 'compact', 'roomy'],
  },
  {
    id: 'appearance.tabCloseButton',
    page: 'appearance',
    label: 'Close button on tabs',
    description: 'Middle click still closes a tab when this is off.',
    keywords: ['x button'],
  },
  {
    id: 'appearance.tabBarSide',
    page: 'appearance',
    label: 'Tab bar position',
    keywords: ['vertical tabs', 'side bar', 'left', 'right', 'top', 'bottom'],
  },
  {
    id: 'appearance.tabRows',
    page: 'appearance',
    label: 'Rows of tabs',
    description: 'For a bar at the top or bottom. Past that it scrolls.',
    keywords: ['many tabs', 'tab rows', 'scrollbar', 'show all tabs', 'wrap', 'lines'],
    keys: ['appearance.tabMaxRows'],
  },
  {
    id: 'appearance.tabOnlyLive',
    page: 'appearance',
    label: 'Only live tabs',
    description: "Hides tabs whose channels are offline. The one you’re on stays.",
    keywords: ['hide offline', 'offline tabs', 'live only', 'filter tabs'],
  },
  {
    id: 'appearance.tabUnread',
    page: 'appearance',
    label: 'Light up tabs',
    description: 'Background tabs light up on new chat, brighter for mentions.',
    keywords: ['unread', 'new messages', 'mentions'],
  },

  // Windows
  {
    id: 'windows.layouts',
    page: 'windows',
    label: 'Window layouts',
    description: 'Where the main window and every pop-out sit, and whether they’re pinned.',
    keywords: ['save layout', 'window positions', 'pop-outs', 'pin', 'workspace'],
  },

  // Sound
  {
    id: 'general.sound.separateStreams',
    page: 'sound',
    label: 'Streams on their own',
    description: 'Windows’ volume mixer shows streams as “cyex deck - streams”.',
    keywords: ['volume mixer', 'mixer', 'entry', 'combine', 'audio', 'windows'],
  },
  {
    id: 'general.sound.streamsDevice',
    page: 'sound',
    label: 'Streams play on',
    description: 'Streams, VODs, clips and stories.',
    keywords: ['output', 'device', 'speakers', 'headphones', 'audio', 'sink'],
  },
  {
    id: 'general.sound.soundsDevice',
    page: 'sound',
    label: 'Other sounds play on',
    description: 'Mentions, highlights, live alerts and whispers.',
    keywords: ['output', 'device', 'speakers', 'headphones', 'chime', 'notification sound', 'test sound'],
  },

  // Hotkeys: each one is listed by `hotkeyEntries`.
  {
    id: 'hotkeys.bindings',
    page: 'hotkeys',
    label: 'Hotkeys',
    description: 'Click a shortcut, then press the new keys.',
    keywords: ['shortcuts', 'keyboard', 'keys', 'rebind', 'global hotkeys'],
  },

  // Plugins
  {
    id: 'plugins.enabled',
    page: 'plugins',
    label: 'Turn on plugins',
    description: 'Lua scripts that add commands, change chat lines and more.',
    keywords: ['lua', 'scripts', 'addons', 'extensions'],
  },
  {
    id: 'plugins.list',
    page: 'plugins',
    label: 'Your plugins',
    description: 'Put a plugin’s folder into the plugins folder, then Look again.',
    keywords: ['plugins folder', 'install', 'permissions', 'lua'],
  },

  // Backup and sharing
  {
    id: 'data:export',
    page: 'data',
    label: 'Export settings',
    description: 'To a file or a share code. Accounts and sign-ins are never included.',
    keywords: ['backup', 'save', 'share code', 'copy'],
  },
  {
    id: 'data:import',
    page: 'data',
    label: 'Import settings',
    description: 'From a file or a share code.',
    keywords: ['load', 'share code', 'restore'],
  },
  {
    id: 'data:chatterino',
    page: 'data',
    label: 'Import from Chatterino',
    description: 'Your Chatterino tabs, in order and with their names.',
    keywords: ['chatterino', 'chatterino7', 'window-layout.json', 'tabs', 'layout', 'migrate'],
  },
  {
    id: 'data:backups',
    page: 'data',
    label: 'Backups',
    description: 'A copy of your settings is kept for each of the last 3 days.',
    keywords: ['restore', 'undo', 'yesterday'],
  },
  {
    id: 'data:cache',
    page: 'data',
    label: 'Picture cache',
    description: 'Emotes, badges and paints you’ve seen, saved on this PC.',
    keywords: ['cache', 'images', 'emotes', 'clear'],
  },
  {
    id: 'data:folder',
    page: 'data',
    label: 'Open settings folder',
    description: 'Where the settings files are.',
    keywords: ['files', 'json', 'appdata'],
  },
];
