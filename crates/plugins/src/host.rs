//! S5 What plugins reach in the app, and the plain data that goes back and forth.
//! The app implements [`Host`]; plugins never see tokens or settings of their own.

use serde::Serialize;
use ts_rs::TS;

/// Chatterino's `c2.LogLevel`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, rename = "PluginLogLevel")]
pub enum Level {
    Debug,
    Info,
    Warning,
    Critical,
}

/// A chat line as plugins see it. Becomes a Lua table.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Line {
    pub id: String,
    pub channel: String,
    /// Unix milliseconds.
    pub time: i64,
    /// `chat`, `action` (`/me`) or `notice`.
    pub kind: &'static str,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<Sender>,
    /// Badge sets, e.g. `moderator`, `subscriber`.
    pub badges: Vec<String>,
    /// What kind of notice: `sub`, `raid`, `timeout`, …
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply: Option<ReplyTo>,
    /// Title of the channel point reward redeemed with it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reward: Option<String>,
    pub flags: Flags,
    /// S7 The plugin that made the line (`kind` is `plugin`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Sender {
    pub id: String,
    pub login: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ReplyTo {
    pub login: String,
    pub text: String,
}

/// Only the true ones show up in Lua.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Flags {
    #[serde(skip_serializing_if = "is_false")]
    pub own: bool,
    /// Loaded from before the chat opened.
    #[serde(skip_serializing_if = "is_false")]
    pub history: bool,
    /// Names the user.
    #[serde(skip_serializing_if = "is_false")]
    pub mention: bool,
    /// The sender's first message here.
    #[serde(skip_serializing_if = "is_false")]
    pub first: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub deleted: bool,
}

fn is_false(b: &bool) -> bool {
    !b
}

/// What a plugin wants done to a line before it shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Change {
    pub hide: bool,
    /// New text; emotes in it are found again.
    pub text: Option<String>,
    /// `#rrggbbaa`, or empty for the usual highlight color.
    pub highlight: Option<String>,
}

impl Change {
    pub fn is_empty(&self) -> bool {
        !self.hide && self.text.is_none() && self.highlight.is_none()
    }
}

/// S9 The tabs as a plugin sees them (Chatterino's `c2.windows`, read only).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Layout {
    /// The tab in front.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<String>,
    pub tabs: Vec<LayoutTab>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct LayoutTab {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Columns left to right, tiles top to bottom.
    pub columns: Vec<Vec<LayoutTile>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct LayoutTile {
    pub id: String,
    /// The channel's login.
    pub channel: String,
    pub name: String,
    pub chat: bool,
    pub player: bool,
    /// F4 A feed tab's name instead of a channel's chat.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feed: Option<String>,
    pub signed_in: bool,
}

/// S8 Something that happened in a chat, after the fact. S12 Also a tab's channel going
/// live or offline, and the tabs changing.
#[derive(Clone, Debug, PartialEq)]
pub enum ChatEvent {
    Appended(Line),
    /// Same id, new content.
    Replaced(Line),
    /// One line (`id`) or all of a user's (`user_id`, a timeout or ban).
    Deleted {
        channel: String,
        id: Option<String>,
        user_id: Option<String>,
    },
    /// A mod cleared the chat.
    Cleared { channel: String },
    /// The channel's login or display name changed on Twitch.
    Renamed { channel: String, old: String, name: String },
    /// S12 A tab's channel went live or offline, seen by the once-a-minute check.
    Live(LiveChange),
    /// S12 Tabs were added, closed, moved or switched, or a tile changed.
    Layout(Layout),
}

/// S12 A channel's stream started or ended.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct LiveChange {
    pub channel: String,
    pub name: String,
    pub live: bool,
    /// The stream's, while live.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game: Option<String>,
}

/// S11 The person whose name was right-clicked. A name in the viewer list comes with its
/// login only.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct NameUser {
    pub login: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// S7 A line a plugin built (`cyex.Message.new`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NewLine {
    /// Final: `p:<plugin>:<its own id>`.
    pub id: String,
    /// Unix ms; now when missing.
    pub time: Option<i64>,
    /// `#rrggbbaa`, or empty for the usual highlight color.
    pub highlight: Option<String>,
    pub pieces: Vec<Piece>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    Text {
        text: String,
        color: Option<PieceColor>,
        style: Option<PieceStyle>,
        tip: Option<String>,
        /// Find emotes, links and @names in it like in a chat line.
        parse: bool,
        link: Option<LineLink>,
    },
    Mention {
        text: String,
        login: String,
    },
    Break,
    /// S10 Addresses with how much sharper than 1x each is, and the size at 1x.
    Image {
        urls: Vec<(String, u8)>,
        size: Option<(u16, u16)>,
        round: bool,
        tip: Option<String>,
        link: Option<LineLink>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PieceColor {
    Link,
    System,
    /// `#rrggbb`, made readable on each theme.
    Rgb(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PieceStyle {
    Bold,
    Italic,
    Button,
}

/// S7 What a click on a piece does: Chatterino's link types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineLink {
    Url(String),
    User(String),
    /// A command or, with SendMessages, a message, as if typed.
    Run(String),
    Jump(String),
    Copy(String),
    Insert(String),
}

/// What a plugin wants done to a line the user is sending.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sent {
    Unchanged,
    Text(String),
    Stop,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ChannelInfo {
    pub login: String,
    pub name: String,
    pub id: String,
    pub broadcaster: bool,
    pub moderator: bool,
    pub vip: bool,
    pub modes: RoomModes,
    pub stream: Option<StreamStatus>,
}

/// Chatterino's `get_room_modes()`.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct RoomModes {
    pub subscriber_only: bool,
    pub unique_chat: bool,
    pub emote_only: bool,
    /// Minutes someone must follow first; absent when anyone may talk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follower_only: Option<i64>,
    /// Seconds between messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slow_mode: Option<u32>,
}

/// Chatterino's `get_stream_status()`.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct StreamStatus {
    pub live: bool,
    pub viewer_count: u64,
    pub title: String,
    pub game_name: String,
    /// Seconds since it went live.
    pub uptime: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Account {
    pub login: String,
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: String,
    /// Already checked against the plugin's hosts.
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub timeout_ms: u64,
    /// Redirects must stay on these too (empty: any public host).
    pub hosts: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HttpResponse {
    /// 0 when no answer came.
    pub status: u16,
    pub data: Vec<u8>,
    pub error: Option<String>,
}

/// S6 Where a plugin's WebSocket goes. The address is checked already.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
}

/// S6 What a plugin sends. Dropping the sender closes the socket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketOut {
    Text(String),
    Binary(Vec<u8>),
}

/// S6 What a socket tells the plugin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketEvent {
    Open,
    Text(String),
    Binary(Vec<u8>),
    /// Also when it never connected: then `code` is `None` and `reason` says why.
    Closed { code: Option<u16>, reason: String },
}

/// S6 Hands a socket's events to its plugin. `false`: the plugin is too far behind and the
/// socket closes; `Closed` always gets through.
pub type Emit = Box<dyn Fn(SocketEvent) -> bool + Send + Sync>;

/// Chatterino's `CompletionRequested` event.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CompletionRequest {
    pub query: String,
    pub full_text_content: String,
    pub cursor_position: usize,
    pub is_first_word: bool,
    pub channel: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Completion {
    pub values: Vec<String>,
    pub hide_others: bool,
}

/// The app's side. Calls come from plugin threads, sometimes while a chat waits on the
/// plugin, so nothing here may wait on chat state that's locked: `send` and `system` only
/// queue their work.
pub trait Host: Send + Sync + 'static {
    fn log(&self, plugin: &str, level: Level, text: &str);
    /// The signed-in default account, `None` when anonymous.
    fn account(&self) -> Option<Account>;
    /// `None` when no open chat shows this channel.
    fn channel(&self, login: &str) -> Option<ChannelInfo>;
    /// The last `n` lines of an open chat, oldest first.
    fn lines(&self, login: &str, n: usize) -> Vec<Line>;
    fn send(&self, plugin: &str, login: &str, text: &str) -> Result<(), String>;
    /// A line only the user sees.
    fn system(&self, plugin: &str, login: &str, text: &str);
    fn http(&self, req: HttpRequest, done: Box<dyn FnOnce(HttpResponse) + Send>);
    /// S6 Runs a socket until `out` closes or the other side does ([`crate::socket::run`]).
    fn socket(&self, req: SocketRequest, out: tokio::sync::mpsc::Receiver<SocketOut>, emit: Emit);
    /// S7 Queues a plugin's line for every open chat of `login`.
    fn add_line(&self, plugin: &str, login: &str, line: NewLine);
    /// S7 Swaps the plugin's own line `line.id` for this one.
    fn replace_line(&self, plugin: &str, login: &str, line: NewLine);
    /// S7 Changes a line that's in already: new text, a highlight, or hidden.
    fn change_line(&self, plugin: &str, login: &str, id: &str, change: Change);
    /// S7 A line by its id; `None` when it's gone.
    fn find_line(&self, login: &str, id: &str) -> Option<Line>;
    /// S9 The tabs and what each shows.
    fn layout(&self) -> Layout;
    /// S13 Switches to the first tab showing `login`, or opens a tab for it. Queued.
    fn open_channel(&self, plugin: &str, login: &str);
    /// S13 Switches to the tab `id`. Queued.
    fn select_tab(&self, plugin: &str, id: &str);
    /// S14 Takes back the plugin's own line `id` from every open chat of `login`.
    fn remove_line(&self, plugin: &str, login: &str, id: &str);
    /// S14 Empties every open chat of `login` on the user's screen (not on Twitch).
    fn clear_lines(&self, plugin: &str, login: &str);
    /// The app's own and Twitch's commands, which plugins can't take over.
    fn reserved(&self, command: &str) -> bool;
    /// Commands, callbacks or menu entries changed.
    fn exports_changed(&self, plugin: &str);
}
