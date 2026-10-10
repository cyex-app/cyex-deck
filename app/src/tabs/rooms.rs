//! 1.10 Every tile that shows chat keeps its room joined, on screen or not, plus the
//! chats the miner sits in (08). The rooms count toward the account's 100.

use cyex_settings::Partition;
use cyex_settings::schema::Tabs;
use tauri::{AppHandle, Manager};

use crate::settings::Settings;

pub fn sync(app: &AppHandle) {
    let tabs: Tabs = app.state::<Settings>().read(Partition::Tabs);
    let mut wanted: Vec<(Option<String>, String)> = tabs
        .tiles()
        .filter(|t| t.show.chat && !t.channel.login.is_empty())
        .map(|t| (t.account.clone(), t.channel.login.clone()))
        .collect();
    wanted.extend(crate::miner::chat_rooms(app));
    crate::chat::hold(app, &wanted);
}
