// V103-13 Settings in the App group: General, Appearance, Windows, Hotkeys, Plugins,
// Backup and sharing.

import type { Entry } from './entry';

export const APP: Entry[] = [
  // General
  {
    id: 'general.chatFontFamily',
    page: 'general',
    label: 'Chat font',
    description: 'These come with the app. Add your own from a font file.',
    keywords: ['typeface', 'text', 'Inter', 'Roboto', 'Open Sans', 'Lato', 'JetBrains Mono'],
  },
  {
    id: 'general.fonts',
    page: 'general',
    label: 'Add a font file',
    description: 'Use your own font for chat.',
    keywords: ['custom font', 'ttf', 'otf', 'woff'],
  },
  {
    id: 'general:installedFont',
    page: 'general',
    label: 'Installed font',
    description: 'Any other font on this PC, by name.',
    keywords: ['chat font', 'Segoe UI', 'Arial', 'Helvetica'],
  },
  { id: 'general.chatFontSize', page: 'general', label: 'Chat font size', keywords: ['text size', 'bigger', 'smaller', 'zoom'] },
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
    description: 'Scrolling glides instead of jumping, everywhere in the app. Applies after a restart.',
    keywords: ['scroll', 'scrolling', 'smooth', 'glide', 'wheel', 'animation', 'restart'],
  },
  {
    id: 'general.minimizeToTray',
    page: 'general',
    label: 'Minimize to tray',
    description: 'Minimizing hides cyex deck in the tray instead of the taskbar.',
    keywords: ['taskbar', 'system tray', 'background'],
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
    id: 'data:backups',
    page: 'data',
    label: 'Backups',
    description: 'A copy of your settings is kept for each of the last 3 days.',
    keywords: ['restore', 'undo', 'yesterday'],
  },
  {
    id: 'data:folder',
    page: 'data',
    label: 'Open settings folder',
    description: 'Where the settings files are.',
    keywords: ['files', 'json', 'appdata'],
  },
];
