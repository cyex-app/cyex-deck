<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="logo/png/mark-dark-128.png">
    <img src="logo/png/mark-light-128.png" width="96" alt="cyex">
  </picture>
</p>

<h1 align="center">cyex deck</h1>

<p align="center">A light Twitch app for Windows: streams, chat, the miner, mod tools and more, without twitch.tv or Chatterino.</p>

> **Private test build (0.1.0).** Windows 10 or 11, 64-bit only. Not affiliated with Twitch. It uses Twitch's internal API the way other third-party clients do, so use it at your own risk.

## What's in it

- **Tabs and tiles:** any number of streams and chats side by side, groups, pop-out windows and saved layouts. Drag a chat to any side of its stream. Layouts rearrange themselves in small windows, and a stream you switch away from keeps playing until another live one is on screen.
- **Player:** live, VODs and clips, with ad blocking, resuming VODs where you left off, making clips and media keys.
- **Chat:** 7TV, BTTV and FFZ emotes, highlights, filters, ignores, search, custom commands, recordings, replies and pins. History from logs.cyex.app when Twitch has none, and `#channel` opens that chat in its own window.
- **Browsing:** home and discover, followed channels, and a usercard for every chatter with their logs and earned badges.
- **Channel points:** a miner that watches channels for points and drops, with bets and streaks.
- **Mod view:** mod tools, the mod log, AutoMod, mass actions and a chat of its own. Mod mode puts delete, timeout and ban in front of every chat line.
- **For streamers:** a stream manager with title, category, stats, alerts, raids, ads, polls, chat and OBS text files.
- **Twitch account:** whispers, notifications and subscriptions.
- **Make it yours:** themes, fonts, streamer mode, chat overlays, global hotkeys and Lua plugins.

## Build it

### What you need

| | Why |
|---|---|
| Windows 10 or 11, 64-bit | The app is Windows-only. |
| [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) | Draws the app. It's already on Windows 11 and on up-to-date Windows 10. |
| [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/visual-cpp-build-tools/), workload **Desktop development with C++** | Rust needs its linker, and Lua is compiled from source. |
| [Rust](https://rustup.rs) 1.90 or newer, MSVC toolchain | The app itself. |
| [Node.js](https://nodejs.org) 22.6 or newer, with npm | The UI. |
| [Git](https://git-scm.com) | To clone the repo. |

You can install all of them with winget, then **open a new terminal** so the new paths load:

```powershell
winget install Git.Git
winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install Rustlang.Rustup
winget install OpenJS.NodeJS.LTS
rustup default stable-x86_64-pc-windows-msvc
```

### Get the code

The repo is private, so you need to be added to it first.

```powershell
git clone https://github.com/cyex-app/cyex-deck.git
cd cyex-deck
npm ci
```

### Run it

```powershell
npm run dev
```

The first start compiles everything, which takes a few minutes. After that, UI changes show up right away, and Rust changes rebuild and restart the app.

### Build the installer

```powershell
npm run build
```

The build creates two files:
- `target\release\cyex-deck.exe`, which runs on its own.
- `target\release\bundle\nsis\cyex deck_0.1.0_x64-setup.exe`, the installer.

The first build downloads NSIS by itself. The installer isn't signed, so Windows SmartScreen may warn about it: click **More info**, then **Run anyway**.

## First start

You can watch streams and read chat without an account. To sign in, open **Settings → Accounts**:

- **Sign in with a code** works right away. It gives you chat, the miner, following, channel points, GIFs and power-ups, but no mod tools, payment methods or subscriptions.
- **Sign in with Twitch** gives you mod tools, payment methods and subscriptions. Afterwards the account card offers the GQL tools too: a code, or a GQL token you paste. It needs a Twitch client ID. Ask for the one already in use, or make your own:
  1. Go to [dev.twitch.tv/console/apps](https://dev.twitch.tv/console/apps) and register an app. Twitch asks for two-factor authentication on your account first.
  2. Fill it in:
     - **Name:** anything.
     - **OAuth Redirect URLs:** `http://localhost`.
     - **Category:** Application Integration.
     - **Client Type:** **Public**.
  3. Paste the client ID into Settings → Accounts.

  For `npm run dev`, you can also copy `.env.example` to `.env` and put the client ID there. `.env` is never committed.

Anything your sign-in can't do is greyed out with a flag that says what it needs, **GQL only** or **Twitch sign-in**. Click the flag to sign in.

## Where your data lives

| What | Where |
|---|---|
| Settings, with daily backups kept for 3 days | `%APPDATA%\cyex-deck\settings\` |
| Client ID | `%APPDATA%\cyex-deck\.env` |
| Plugins | `%APPDATA%\cyex-deck\plugins\` |
| Sign-in tokens | Windows Credential Manager, never in files |
| Twitch's pages in the app | `%LOCALAPPDATA%\cyex-deck\twitch\` |
| Browser cache | `%LOCALAPPDATA%\app.cyex.deck\` |

Uninstalling the app leaves all of this in place. To start fresh, close the app and delete these folders.

The app used to have another name. On its first start under this one, it copies the old folders and sign-ins over and leaves the old ones as they were. Once everything works, the old folders (`cyex-viewer` and `app.cyex.viewer`) and the old `cyex-viewer` entries in the Credential Manager can go, and the old app can be uninstalled.

## Good to know

- `cyex-deck.exe --safe-mode` starts the app without running any plugin.
- To write a plugin, see [docs/plugins.md](docs/plugins.md).
- Settings → Backup and sharing exports your settings to a file or copies them as a share code.

## Problems

| Problem | What to do |
|---|---|
| `link.exe` or `cl.exe` not found, or the `mlua`/`lua-src` build fails | Build Tools or the C++ workload is missing, or Rust uses the GNU toolchain. Run `rustup default stable-x86_64-pc-windows-msvc`, then try again in a new terminal. |
| `npm run dev` says port 5173 is busy | Close the other dev server. |
| The window stays blank | Install or update the WebView2 Runtime. |
| Things broke after adding a plugin | Start with `--safe-mode`, then check the plugin's console (Settings → Plugins → `>_`). |

Found a bug? Open an issue in this repo. Include what you did, what happened, what you expected, and a screenshot if it helps.

## For developers

```powershell
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run check
npm run test:ui
```

`cargo test` also regenerates the TypeScript types in `ui/src/ipc/bindings/`.

| Folder | What's in it |
|---|---|
| `app/` | The Tauri app: commands, windows, and the glue between the crates and the UI. |
| `crates/` | `core`, `settings`, `twitch`, `auth`, `chat`, `player`, `emotes`, `miner`, `eventsub`, `moderation`, `plugins`. |
| `ui/` | Svelte 5 frontend. |
| `docs/` | Specs per module, plus [ARCHITECTURE](docs/ARCHITECTURE.md), [DECISIONS](docs/DECISIONS.md) and [PHASES](docs/PHASES.md). |
| `logo/` | App icon and logo. |
