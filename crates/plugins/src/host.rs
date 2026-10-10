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
    /// The app's own and Twitch's commands, which plugins can't take over.
    fn reserved(&self, command: &str) -> bool;
    /// Commands, callbacks or menu entries changed.
    fn exports_changed(&self, plugin: &str);
}
