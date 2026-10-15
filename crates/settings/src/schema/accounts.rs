use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

/// Signed-in accounts (1.3, 1.23). Tokens are never here: they live in the Windows
/// Credential Manager. This file only says which tokens exist and what they allow.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Accounts {
    pub list: Vec<Account>,
    /// Used for new tabs and the top bar. Always one of `list` while it isn't empty.
    pub default_id: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Account {
    pub user_id: String,
    pub login: String,
    pub display_name: String,
    /// Profile picture URL, small size.
    pub avatar: String,
    /// Token for this app's own client ID: Helix, IRC, EventSub.
    pub helix: Option<TokenMeta>,
    /// First-party token for GQL-only tools (1.54).
    pub gql: Option<TokenMeta>,
    /// V1051-02 Consent to use 7TV with this account (date given, `yyyy-mm-dd`). Not a secret.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seventv: Option<SevenTvConsent>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct SevenTvConsent {
    pub since: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct TokenMeta {
    pub source: TokenSource,
    pub scopes: Vec<String>,
    /// Unix seconds. `None` means it doesn't expire.
    #[ts(type = "number | null")]
    pub expires_at: Option<i64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TokenSource {
    /// Signed in through twitch.tv/activate.
    #[default]
    Device,
    /// Pasted by hand (1.4).
    Pasted,
}

impl Accounts {
    pub fn get(&self, user_id: &str) -> Option<&Account> {
        self.list.iter().find(|a| a.user_id == user_id)
    }

    pub fn get_mut(&mut self, user_id: &str) -> Option<&mut Account> {
        self.list.iter_mut().find(|a| a.user_id == user_id)
    }

    pub fn default_account(&self) -> Option<&Account> {
        self.default_id.as_deref().and_then(|id| self.get(id))
    }
}

impl Schema for Accounts {
    fn sanitize(&mut self) {
        let mut seen = Vec::new();
        self.list.retain(|a| {
            let keep = !a.user_id.is_empty() && !seen.contains(&a.user_id);
            seen.push(a.user_id.clone());
            keep
        });
        if self.default_account().is_none() {
            self.default_id = self.list.first().map(|a| a.user_id.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(id: &str) -> Account {
        Account {
            user_id: id.into(),
            login: format!("user{id}"),
            ..Default::default()
        }
    }

    #[test]
    fn sanitize_drops_blank_and_duplicate_accounts() {
        let mut a = Accounts {
            list: vec![account("1"), account(""), account("1"), account("2")],
            default_id: Some("2".into()),
        };
        a.sanitize();
        let ids: Vec<_> = a.list.iter().map(|a| a.user_id.as_str()).collect();
        assert_eq!(ids, ["1", "2"]);
        assert_eq!(a.default_id.as_deref(), Some("2"));
    }

    #[test]
    fn default_falls_back_to_the_first_account() {
        let mut a = Accounts {
            list: vec![account("1"), account("2")],
            default_id: Some("9".into()),
        };
        a.sanitize();
        assert_eq!(a.default_id.as_deref(), Some("1"));
        let mut empty = Accounts {
            list: vec![],
            default_id: Some("1".into()),
        };
        empty.sanitize();
        assert_eq!(empty.default_id, None);
    }

    #[test]
    fn an_old_file_without_7tv_still_loads() {
        let old = r#"{"list":[{"userId":"1","login":"a","displayName":"A","avatar":""}],"defaultId":"1"}"#;
        let a: Accounts = serde_json::from_str(old).unwrap();
        assert!(a.list[0].seventv.is_none());
        let out = serde_json::to_string(&a).unwrap();
        assert!(!out.contains("seventv"));
    }
}
