//! 1.8 What happens to a line typed in a chat: `//` and custom commands are filled in
//! (Chatty's syntax), the app's own commands run (`builtin`), and the rest goes to
//! Twitch. Menu entries (S1) and automations (S4) run through here too.

mod builtin;
mod cosmetics;
mod seventv_edit;
pub use builtin::{block_user, reserved};
pub mod complete;
mod host;
pub mod registry;
pub mod timers;

use std::sync::{Arc, Mutex};

use cyex_chat::automate::Automations;
use cyex_chat::commands::{self, CommandList, Params};
use cyex_chat::model::{ChatMessage, NoticeKind};
use cyex_settings::Partition;
use cyex_settings::schema::{Account, Commands};
use cyex_twitch::gql::ChannelInfo;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::{Chat, Identity, RoomKey};
use crate::cosmetics::VarScope;
use crate::emotes::edit::{EditDone, EditOp};
use crate::settings::Settings;
use host::AppHost;

/// Commands calling commands stop here.
const MAX_DEPTH: u32 = 8;

#[derive(Default)]
pub struct CommandsState {
    list: Mutex<Arc<CommandList>>,
    pub(super) autos: Mutex<Automations>,
}

pub fn init(app: &AppHandle) {
    app.manage(CommandsState::default());
    rebuild(app);
    timers::restore(app);
}

/// After the commands settings changed.
pub fn rebuild(app: &AppHandle) {
    let Some(state) = app.try_state::<CommandsState>() else {
        return;
    };
    let c: Commands = app.state::<Settings>().read(Partition::Commands);
    *state.list.lock().unwrap_or_else(|p| p.into_inner()) = Arc::new(CommandList::new(&c.command_lines()));
    state.autos.lock().unwrap_or_else(|p| p.into_inner()).update(&c);
}

fn list(app: &AppHandle) -> Arc<CommandList> {
    app.state::<CommandsState>()
        .list
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

/// Where a line runs.
#[derive(Clone, Debug)]
pub struct Place {
    pub view: Option<u32>,
    pub key: RoomKey,
    /// V1051-04 Started by an automation (S4), down to the commands it runs.
    pub automation: bool,
    /// Typed or picked in this chat view, so its window acts on a `UiAction` (a 7TV sign-in
    /// pop-up). Not for usercard buttons, timers, `/runin` or automations: they get a line.
    pub ui: bool,
}

impl Place {
    pub fn channel(&self) -> &str {
        &self.key.1
    }

    pub fn account(&self) -> Option<&str> {
        match &self.key.0 {
            Identity::Account(id) => Some(id),
            Identity::Anon => None,
        }
    }
}

/// What the window should do after a line ran.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum UiAction {
    /// F1
    Search {
        query: String,
    },
    Popout,
    /// V1051-03 Open the 7TV paint and badge pop-up.
    Cosmetics,
    /// `/paint`, `/badge` or `/combo` needs 7TV's sign-in: the sign-in pop-up, then `command`
    /// runs again. `again`: the session ran out.
    SevenTvSignIn { command: String, again: bool },
    /// I106-3 /add, /remove or /rename worked: the chat view shows the Undo toast.
    SevenTvEdited { op: EditOp, done: EditDone },
}

/// S1 Who and what a menu entry was picked on.
#[derive(Clone, Debug, Default, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct MenuContext {
    pub login: Option<String>,
    pub user_id: Option<String>,
    pub display: Option<String>,
    pub msg_id: Option<String>,
    pub msg: Option<String>,
    #[ts(type = "number | null")]
    pub msg_time: Option<i64>,
}

/// A line typed in a chat view.
pub async fn input(
    app: &AppHandle,
    view: u32,
    text: &str,
    reply_to: Option<String>,
) -> Result<Option<UiAction>, String> {
    let place = place_of(app, view, true)?;
    line(app, &place, text, reply_to, true, 0).await
}

/// S1 A menu entry or usercard button: its command is filled in, then run.
pub async fn menu(
    app: &AppHandle,
    view: u32,
    command: &str,
    ctx: MenuContext,
) -> Result<Option<UiAction>, String> {
    menu_at(app, view, command, ctx, true).await
}

/// `ui`: the caller is the chat view itself (see `Place::ui`).
async fn menu_at(
    app: &AppHandle,
    view: u32,
    command: &str,
    ctx: MenuContext,
    ui: bool,
) -> Result<Option<UiAction>, String> {
    let place = place_of(app, view, ui)?;
    let mut p = params(app, &place, ctx.login.as_deref().unwrap_or(""));
    for (k, v) in MENU_VALUES.into_iter().zip([&ctx.login, &ctx.display, &ctx.user_id, &ctx.msg_id, &ctx.msg]) {
        if let Some(v) = v {
            p.set(k, v.clone());
        }
    }
    if let Some(t) = ctx.msg_time {
        p.set(MSG_TIME, t.to_string());
    }
    let filled = fill(app, &place, command, p)
        .await?
        .ok_or("Something this needs is missing.")?;
    line(app, &place, &filled, None, true, 0).await
}

/// S4 An automation went off in a room.
pub async fn automation(
    app: &AppHandle,
    key: RoomKey,
    response: &str,
    args: &str,
    nick: &str,
    count: u64,
    reply_to: Option<String>,
) {
    let place = Place {
        view: None,
        key,
        automation: true,
        ui: false,
    };
    let mut p = params(app, &place, args);
    let [nick_key, count_key] = AUTOMATION_VALUES;
    p.set(nick_key, nick);
    p.set(count_key, count.to_string());
    let result = match fill(app, &place, response, p).await {
        Ok(Some(filled)) => line(app, &place, &filled, reply_to, true, 0).await.map(drop),
        Ok(None) => Ok(()),
        Err(e) => Err(e),
    };
    if let Err(e) = result {
        local(app, &place.key, NoticeKind::Error, &format!("Automation: {e}"));
    }
}

/// S1 A usercard button: runs in an open view of the card's channel, a signed-in one first.
pub async fn menu_in(
    app: &AppHandle,
    channel: &str,
    command: &str,
    ctx: MenuContext,
) -> Result<Option<UiAction>, String> {
    let view = {
        let chat = app.state::<Chat>();
        let inner = chat.lock();
        let mut open: Vec<(u32, bool)> = inner
            .views
            .iter()
            .filter(|(_, (key, _))| key.1 == channel)
            .map(|(id, (key, _))| (*id, matches!(key.0, Identity::Account(_))))
            .collect();
        open.sort_by_key(|(id, signed)| (!signed, *id));
        open.first().map(|(id, _)| *id)
    };
    menu_at(app, view.ok_or("Open the chat first.")?, command, ctx, false).await
}

fn place_of(app: &AppHandle, view: u32, ui: bool) -> Result<Place, String> {
    let chat = app.state::<Chat>();
    let key = chat
        .lock()
        .views
        .get(&view)
        .map(|v| v.0.clone())
        .ok_or("This chat is closed.")?;
    Ok(Place {
        view: Some(view),
        key,
        automation: false,
        ui,
    })
}

/// Runs one line. `custom` lets it be a custom command.
fn line<'a>(
    app: &'a AppHandle,
    place: &'a Place,
    text: &'a str,
    reply_to: Option<String>,
    custom: bool,
    depth: u32,
) -> std::pin::Pin<Box<dyn Future<Output = Result<Option<UiAction>, String>> + Send + 'a>> {
    Box::pin(async move {
        let text = text.trim();
        if text.is_empty() {
            return Ok(None);
        }
        if depth > MAX_DEPTH {
            return Err("Too many commands in a row.".into());
        }
        if let Some(body) = text.strip_prefix("//") {
            let p = params(app, place, "");
            let filled = fill(app, place, &format!("/{body}"), p)
                .await?
                .ok_or("Something this needs is missing.")?;
            return line(app, place, &filled, reply_to, false, depth + 1).await;
        }
        // F14 A dot starts a command only when the app knows it; ".gg" goes out as typed.
        let Some((name, args)) = super::typed::command(text, |n| known(app, place, n, custom)) else {
            return send(app, place, text, reply_to).await.map(|()| None);
        };
        let name = name.to_ascii_lowercase();
        if custom && let Some(body) = list(app).command(&name, place.channel()) {
            let p = params(app, place, args);
            let filled = run_body(app, place, body, p)
                .await?
                .ok_or_else(|| format!("/{name} needs more after it."))?;
            return line(app, place, &filled, reply_to, false, depth + 1).await;
        }
        // S5 A plugin's command; what it returns runs as if typed.
        if let Some(p) = crate::plugins::calls::command_owner(app, &name) {
            let out = crate::plugins::calls::command(&p, &name, args, place.channel())
                .await
                .map_err(|e| format!("/{name}: {e}"))?;
            return match out {
                Some(next) => line(app, place, &next, reply_to, false, depth + 1).await,
                None => Ok(None),
            };
        }
        builtin::run(app, place, &name, args, text, reply_to, depth).await
    })
}

/// F14 A command by that (lower-case) name exists here: the app's or Twitch's, a plugin's
/// (S5), or, where `custom`, your own for this channel.
fn known(app: &AppHandle, place: &Place, name: &str, custom: bool) -> bool {
    reserved(name)
        || crate::plugins::calls::command_owner(app, name).is_some()
        || (custom && list(app).command(name, place.channel()).is_some())
}

/// Plain text (or `/me`) to Twitch, after plugins had their say (S5).
async fn send(app: &AppHandle, place: &Place, text: &str, reply_to: Option<String>) -> Result<(), String> {
    let text = crate::plugins::calls::sending(app, place.channel(), text).await?;
    super::send_to(app, &place.key, &text, reply_to)
}

/// V1051-04 What a menu entry or user card button adds, in the order of `menu()`'s context
/// (the registry checks these names), and `msg-time` after them.
const MENU_VALUES: [&str; 5] = ["nick", "display-nick", "user-id", "msg-id", "msg"];
const MSG_TIME: &str = "msg-time";
/// What an automation adds.
const AUTOMATION_VALUES: [&str; 2] = ["nick", "count"];

/// The values every command can read.
fn params(app: &AppHandle, place: &Place, args: &str) -> Params {
    let live = crate::tabs::live::snapshot(app)
        .into_iter()
        .find(|c| c.login == place.channel());
    let me = place
        .account()
        .and_then(|id| crate::auth::accounts(app).get(id).cloned());
    let mut p = values(place.channel(), &super::open_channels(app), live.as_ref(), me.as_ref());
    p.set_args(args);
    p
}

/// The pure part of `params`: the channel, the open chats, the stream and the account.
fn values(channel: &str, open: &[String], live: Option<&ChannelInfo>, me: Option<&Account>) -> Params {
    let mut p = Params::default();
    p.set("chan", channel);
    p.set("stream", channel);
    p.set("chans", open.join(" "));
    if let Some(info) = live {
        p.set("streamname", info.name.clone());
        p.set("streamdisplayname", info.name.clone());
        match &info.stream {
            Some(s) => {
                p.set("streamtitle", s.title.clone());
                p.set("streamgame", s.game.clone().unwrap_or_default());
                p.set("streamviewers", s.viewers.to_string());
                p.set(
                    "streamstatus",
                    format!("{} ({})", s.title, s.game.clone().unwrap_or_default()),
                );
            }
            None => p.set("streamstatus", "Offline"),
        }
    }
    if let Some(a) = me {
        p.set("my-nick", a.login.clone());
        p.set("my-display-nick", a.display_name.clone());
        p.set("my-user-id", a.user_id.clone());
    }
    p
}

/// Fills in a body typed out as text.
async fn fill(app: &AppHandle, place: &Place, body: &str, p: Params) -> Result<Option<String>, String> {
    let items = commands::parse(body).map_err(|e| format!("Can’t read that command: {e}"))?;
    run_body(app, place, Arc::new(items), p).await
}

/// `$request` waits on the network, so bodies run off the async threads.
async fn run_body(
    app: &AppHandle,
    place: &Place,
    body: Arc<commands::Items>,
    p: Params,
) -> Result<Option<String>, String> {
    let mut host = AppHost {
        app: app.clone(),
        list: list(app),
        channel: place.channel().to_owned(),
        runtime: tokio::runtime::Handle::current(),
        vars: VarScope::new(place.account().map(str::to_owned), place.automation),
    };
    let (out, notes) = tokio::task::spawn_blocking(move || {
        let out = commands::run(&body, &p, &mut host);
        (out, host.vars.notes)
    })
    .await
    .map_err(|e| e.to_string())?;
    for n in notes {
        local(app, &place.key, NoticeKind::Error, &n);
    }
    Ok(out)
}

/// A line only you see, in every view of the room.
pub fn local(app: &AppHandle, key: &RoomKey, kind: NoticeKind, text: &str) {
    super::local_line(app, key, kind, text);
}

/// S4 Checks a live message against the automations.
pub fn automate(app: &AppHandle, key: &RoomKey, m: &ChatMessage) {
    // V107-A9 No automations (or anonymous): no live-list scan either.
    if key.0 == Identity::Anon {
        return;
    }
    let state = app.state::<CommandsState>();
    if state.autos.lock().unwrap_or_else(|p| p.into_inner()).is_empty() {
        return;
    }
    // Read without the automations lock held, so no new lock order.
    let live = crate::tabs::live::is_live(app, &key.1);
    let fired = state.autos.lock().unwrap_or_else(|p| p.into_inner()).check(&key.1, m, live, super::now_ms());
    let Some(f) = fired else { return };
    let nick = m.sender.as_ref().map(|s| s.login.clone()).unwrap_or_default();
    let (app, key) = (app.clone(), key.clone());
    let _ = app
        .state::<Settings>()
        .update::<Commands>(&app, Partition::Commands, |c| {
            match c.automations.get_mut(f.rule) {
                Some(a) => {
                    a.count = f.count;
                    true
                }
                None => false,
            }
        });
    tauri::async_runtime::spawn(async move {
        automation(&app, key, &f.response, &f.args, &nick, f.count, f.reply_to).await;
    });
}
