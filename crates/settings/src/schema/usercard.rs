//! V111-13 What a usercard shows. Every piece can be turned off to keep the card simple;
//! an item that is off is not looked up either. All on by default, and an old file without
//! a field reads it as on.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct UsercardShow {
    /// When the account was made.
    pub created: bool,
    pub followers: bool,
    /// Sub status and months (api.ivr.fi, D23).
    pub sub: bool,
    /// Who gifted the sub, under the sub status.
    pub gifter: bool,
    /// When they followed this channel (only where an account here moderates it).
    pub follow_age: bool,
    /// Earlier names (logs.cyex.app).
    pub names: bool,
    /// When they last went live, with that stream's title (api.ivr.fi).
    pub last_live: bool,
    /// Their chat colour as hex, with a copy button.
    pub chat_color: bool,
    /// Staff, Partner and Affiliate.
    pub chips: bool,
    /// The Verified bot chip (api.ivr.fi).
    pub verified_bot: bool,
    /// Every Twitch badge they own in the channel (V102-13).
    pub badges: bool,
    /// The 7TV paint's name under theirs.
    pub paint: bool,
    /// The 7TV profile button (V108-C16).
    pub seventv_profile: bool,
    /// Their user ID with a copy button.
    pub user_id: bool,
    /// Bans and timeouts in other channels you moderate (V108-N16).
    pub other_mod_counts: bool,
    /// Where they are mod, VIP, founder or artist, from roles.tv.
    pub roles: bool,
    /// The Follow button (1.11), for the signed-in account.
    pub follow: bool,
    /// Timeouts, bans, warnings and comments tabs where you moderate (M3).
    pub mod_tabs: bool,
}

impl Default for UsercardShow {
    fn default() -> Self {
        Self {
            created: true,
            followers: true,
            sub: true,
            gifter: true,
            follow_age: true,
            names: true,
            last_live: true,
            chat_color: true,
            chips: true,
            verified_bot: true,
            badges: true,
            paint: true,
            seventv_profile: true,
            user_id: true,
            other_mod_counts: true,
            roles: true,
            follow: true,
            mod_tabs: true,
        }
    }
}

impl UsercardShow {
    /// Whether the card asks api.ivr.fi for the user (last live, the bot flag). The chat colour
    /// comes from chat or the Twitch profile, so it doesn't need it.
    pub fn needs_ivr_user(&self) -> bool {
        self.last_live || self.verified_bot
    }

    /// Whether the card asks api.ivr.fi for the sub status (the gifter comes with it).
    pub fn needs_subage(&self) -> bool {
        self.sub
    }

    /// Whether the card asks roles.tv.
    pub fn needs_roles(&self) -> bool {
        self.roles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v111_13_everything_shows_by_default_and_old_files_read_as_on() {
        let d = UsercardShow::default();
        assert!(d.created && d.followers && d.sub && d.gifter && d.follow_age && d.names);
        assert!(d.last_live && d.chat_color && d.chips && d.verified_bot && d.badges && d.paint);
        assert!(d.seventv_profile && d.user_id && d.other_mod_counts && d.roles && d.follow && d.mod_tabs);
        let old: UsercardShow = serde_json::from_str("{}").unwrap();
        assert_eq!(old, d);
        let one: UsercardShow = serde_json::from_str(r#"{"lastLive":false}"#).unwrap();
        assert!(!one.last_live && one.verified_bot);
    }

    #[test]
    fn v111_13_off_means_no_lookup() {
        let mut s = UsercardShow::default();
        assert!(s.needs_ivr_user() && s.needs_subage() && s.needs_roles());
        s.last_live = false;
        assert!(s.needs_ivr_user(), "the bot chip still needs it");
        s.verified_bot = false;
        assert!(!s.needs_ivr_user());
        s.chat_color = true;
        assert!(!s.needs_ivr_user(), "the colour never needs ivr");
        s.sub = false;
        s.gifter = true;
        assert!(!s.needs_subage(), "the gifter comes with the sub status");
        s.roles = false;
        assert!(!s.needs_roles());
    }
}
