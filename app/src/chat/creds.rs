//! IRC login for an account, read fresh on every connect so refreshed tokens are used.

use cyex_auth::chat_token;
use cyex_chat::irc::{Credentials, Login};
use tauri::{AppHandle, Manager};

use crate::auth::{Auth, accounts};

pub struct AccountCreds {
    app: AppHandle,
    user_id: String,
}

impl AccountCreds {
    pub fn new(app: AppHandle, user_id: String) -> Self {
        Self { app, user_id }
    }
}

impl Credentials for AccountCreds {
    fn login(&self) -> Option<Login> {
        let list = accounts(&self.app);
        let account = list.get(&self.user_id);
        let token = account
            .and_then(|a| chat_token(a, &self.app.state::<Auth>().vault))
            .map(|s| s.access)
            // No usable token: log in with none, so the room shows "sign in again".
            .unwrap_or_default();
        Some(Login {
            nick: account.map_or_else(|| self.user_id.clone(), |a| a.login.clone()),
            token,
        })
    }
}
