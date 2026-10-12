//! 1.10 Every tile that shows chat keeps its room joined, on screen or not, plus the
//! chats the miner sits in (08). The rooms count toward the account's 100. V102-26 Tiles
//! that show only the stream join nothing; their role comes from Twitch's answer instead.

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

    let accounts = crate::auth::accounts(app);
    let stream_only = tabs
        .tiles()
        .filter(|t| t.show.player && !t.show.chat && !t.channel.login.is_empty() && !t.channel.id.is_empty())
        .filter_map(|t| {
            let account = t.account.as_deref().filter(|a| accounts.get(a).is_some())?;
            Some((account.to_owned(), t.channel.login.clone(), t.channel.id.clone()))
        })
        .collect();
    crate::chat::roles::watch(app, stream_only);
}
