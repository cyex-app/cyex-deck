//! What the UI gets for each account: settings data merged with live token state.

use cyex_settings::schema::{Accounts, TokenSource};
use serde::Serialize;
use ts_rs::TS;

use crate::{Feature, SevenTvTokens, TokenKind, TokenState, Vault, caps, scopes, seventv};

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AccountView {
    pub user_id: String,
    pub login: String,
    pub display_name: String,
    pub avatar: String,
    pub is_default: bool,
    pub helix: Option<TokenView>,
    pub gql: Option<TokenView>,
    /// Helix scopes the token lacks. Tools that need them stay hidden.
    pub missing_scopes: Vec<String>,
    pub features: Vec<Feature>,
    pub seventv: SevenTvView,
}

/// V1051-02 Where an account stands with 7TV. Never the token.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SevenTvView {
    pub consent: bool,
    pub signed_in: bool,
    /// The session's last day, `yyyy/mm/dd`; `None` when signed out.
    pub until: Option<String>,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TokenView {
    pub source: TokenSource,
    /// Unix seconds. `None` means it doesn't expire.
    #[ts(type = "number | null")]
    pub expires_at: Option<i64>,
    pub state: TokenState,
}

pub fn views(accounts: &Accounts, vault: &Vault, seventv: &SevenTvTokens) -> Vec<AccountView> {
    accounts
        .list
        .iter()
        .map(|a| {
            let token = |kind: TokenKind| {
                kind.meta(a).map(|m| TokenView {
                    source: m.source,
                    expires_at: m.expires_at,
                    state: vault.state(&a.user_id, kind),
                })
            };
            let helix = token(TokenKind::Helix);
            let gql = token(TokenKind::Gql);
            let usable = |t: &Option<TokenView>| t.as_ref().is_some_and(|t| t.state.usable());
            let helix_scopes = a
                .helix
                .as_ref()
                .filter(|_| usable(&helix))
                .map(|m| m.scopes.as_slice());
            let token = seventv.get(&a.user_id);
            AccountView {
                seventv: SevenTvView {
                    consent: a.seventv.is_some(),
                    signed_in: token.is_some(),
                    until: token.and_then(|t| t.expires()).map(seventv::date),
                },
                user_id: a.user_id.clone(),
                login: a.login.clone(),
                display_name: a.display_name.clone(),
                avatar: a.avatar.clone(),
                is_default: accounts.default_id.as_deref() == Some(a.user_id.as_str()),
                missing_scopes: a
                    .helix
                    .as_ref()
                    .map(|m| scopes::missing(&m.scopes))
                    .unwrap_or_default(),
                features: caps::features(helix_scopes, usable(&gql)),
                helix,
                gql,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cyex_settings::schema::{Account, TokenMeta};

    use super::*;
    use crate::secrets::{MemoryStore, Secret};

    #[test]
    fn dead_tokens_unlock_nothing() {
        let accounts = Accounts {
            list: vec![Account {
                user_id: "1".into(),
                helix: Some(TokenMeta {
                    scopes: scopes::HELIX.iter().map(|s| (*s).to_owned()).collect(),
                    ..Default::default()
                }),
                gql: Some(TokenMeta::default()),
                ..Default::default()
            }],
            default_id: Some("1".into()),
        };
        let vault = Vault::new(Arc::new(MemoryStore::default()));
        let tokens = SevenTvTokens::new(Arc::new(MemoryStore::default()));
        let secret = Secret {
            access: "a".into(),
            refresh: None,
            client_id: "c".into(),
        };
        vault.put("1", TokenKind::Helix, secret).unwrap();

        let v = &views(&accounts, &vault, &tokens)[0];
        assert!(v.is_default && v.missing_scopes.is_empty());
        assert!(v.features.contains(&Feature::SendChat));
        // The GQL token isn't in the credential store, so GQL tools stay hidden.
        assert_eq!(v.gql.as_ref().unwrap().state, TokenState::Missing);
        assert!(!v.features.contains(&Feature::Miner));

        vault.mark("1", TokenKind::Helix, TokenState::Invalid);
        assert!(views(&accounts, &vault, &tokens)[0].features.is_empty());
    }
}
