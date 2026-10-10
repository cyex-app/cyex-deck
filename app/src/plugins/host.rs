//! S5 The app as plugins reach it. Sending and system lines go through a task, because a
//! plugin may call them while a chat waits on its Message callback.

use cyex_chat::model::NoticeKind;
use cyex_plugins::{
    Account, ChannelInfo, Host, HttpRequest, HttpResponse, Level, Line, RoomModes, StreamStatus,
};
use tauri::{AppHandle, Manager};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::{PluginState, hook, logs};
use crate::chat::peek;

pub struct AppHost {
    pub app: AppHandle,
}

impl Host for AppHost {
    fn log(&self, plugin: &str, level: Level, text: &str) {
        logs::push(&self.app, plugin, level, text);
    }

    fn account(&self) -> Option<Account> {
        let accounts = crate::auth::accounts(&self.app);
        accounts.default_account().map(|a| Account {
            login: a.login.clone(),
            id: a.user_id.clone(),
            name: a.display_name.clone(),
        })
    }

    fn channel(&self, login: &str) -> Option<ChannelInfo> {
        let (_, st) = peek::state(&self.app, login)?;
        let live = crate::tabs::live::snapshot(&self.app)
            .into_iter()
            .find(|c| c.login == login);
        let own = st.own.as_ref();
        Some(ChannelInfo {
            login: login.to_owned(),
            name: live.as_ref().map_or_else(|| login.to_owned(), |c| c.name.clone()),
            id: st.channel_id.clone().unwrap_or_default(),
            broadcaster: own.is_some_and(|o| o.broadcaster),
            moderator: own.is_some_and(|o| o.moderator),
            vip: own.is_some_and(|o| o.vip),
            modes: RoomModes {
                subscriber_only: st.subs_only,
                unique_chat: st.unique,
                emote_only: st.emote_only,
                follower_only: st.followers_only.map(i64::from),
                slow_mode: (st.slow > 0).then_some(st.slow),
            },
            stream: live.and_then(|c| c.stream).map(|s| StreamStatus {
                live: true,
                viewer_count: u64::from(s.viewers),
                title: s.title,
                game_name: s.game.unwrap_or_default(),
                uptime: OffsetDateTime::parse(&s.started_at, &Rfc3339)
                    .map(|t| (OffsetDateTime::now_utc() - t).whole_seconds().max(0))
                    .unwrap_or(0),
            }),
        })
    }

    fn lines(&self, login: &str, n: usize) -> Vec<Line> {
        peek::lines(&self.app, login, n)
            .iter()
            .map(|m| hook::to_line(login, m))
            .collect()
    }

    fn send(&self, plugin: &str, login: &str, text: &str) -> Result<(), String> {
        let (app, plugin, login, text) = (
            self.app.clone(),
            plugin.to_owned(),
            login.to_owned(),
            text.to_owned(),
        );
        tauri::async_runtime::spawn(async move {
            let sent = peek::room(&app, &login)
                .ok_or_else(|| format!("The chat of {login} isn't open."))
                .and_then(|key| crate::chat::send_to(&app, &key, &text, None));
            if let Err(e) = sent {
                logs::push(&app, &plugin, Level::Warning, &format!("send_message: {e}"));
            }
        });
        Ok(())
    }

    fn system(&self, plugin: &str, login: &str, text: &str) {
        let (app, plugin, login, text) = (
            self.app.clone(),
            plugin.to_owned(),
            login.to_owned(),
            text.to_owned(),
        );
        tauri::async_runtime::spawn(async move {
            // Named, so a plugin can't pass for the app or Twitch.
            let line = format!("{}: {text}", super::name_of(&app, &plugin));
            if let Some(key) = peek::room(&app, &login) {
                crate::chat::local_line(&app, &key, NoticeKind::Info, &line);
            }
        });
    }

    fn http(&self, req: HttpRequest, done: Box<dyn FnOnce(HttpResponse) + Send>) {
        let client = self.app.state::<PluginState>().http.clone();
        tauri::async_runtime::spawn(async move {
            done(cyex_plugins::fetch::fetch(&client, req).await);
        });
    }

    fn reserved(&self, command: &str) -> bool {
        crate::chat::run::reserved(command)
    }

    fn exports_changed(&self, _plugin: &str) {
        super::exports_changed(&self.app);
    }
}
