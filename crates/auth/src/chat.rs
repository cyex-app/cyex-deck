//! Which token logs an account into chat (1.8). The Helix token when it has the chat
//! scopes, else the GQL token: first-party tokens work on IRC too, so GQL-only accounts
//! can chat (phase 3 decision, `DECISIONS.md`).

use cyex_settings::schema::Account;

use crate::{Secret, TokenKind, Vault};

const READ: &str = "chat:read";
const WRITE: &str = "chat:edit";

/// The token kind to use for chat, if any is usable.
pub fn chat_token_kind(account: &Account, vault: &Vault) -> Option<TokenKind> {
    let usable = |kind| vault.state(&account.user_id, kind).usable() && vault.secret(&account.user_id, kind).is_some();
    let helix_ok = account
        .helix
        .as_ref()
        .is_some_and(|m| m.scopes.iter().any(|s| s == READ) && m.scopes.iter().any(|s| s == WRITE));
    if helix_ok && usable(TokenKind::Helix) {
        Some(TokenKind::Helix)
    } else if account.gql.is_some() && usable(TokenKind::Gql) {
        Some(TokenKind::Gql)
    } else {
        None
    }
}

pub fn chat_token(account: &Account, vault: &Vault) -> Option<Secret> {
    chat_token_kind(account, vault).and_then(|k| vault.secret(&account.user_id, k))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cyex_settings::schema::TokenMeta;

    use super::*;
    use crate::TokenState;
    use crate::secrets::MemoryStore;

    fn secret(access: &str) -> Secret {
        Secret {
            access: access.into(),
            refresh: None,
            client_id: "c".into(),
        }
    }

    fn meta(scopes: &[&str]) -> Option<TokenMeta> {
        Some(TokenMeta {
            scopes: scopes.iter().map(|s| (*s).to_owned()).collect(),
            ..Default::default()
        })
    }

    #[test]
    fn prefers_helix_with_chat_scopes_then_gql() {
        let vault = Vault::new(Arc::new(MemoryStore::default()));
        let mut a = Account {
            user_id: "1".into(),
            helix: meta(&["chat:read", "chat:edit"]),
            gql: meta(&[]),
            ..Default::default()
        };
        vault.put("1", TokenKind::Helix, secret("h")).unwrap();
        vault.put("1", TokenKind::Gql, secret("g")).unwrap();
        assert_eq!(chat_token(&a, &vault).unwrap().access, "h");

        a.helix = meta(&["user:read:follows"]);
        assert_eq!(chat_token(&a, &vault).unwrap().access, "g");

        vault.mark("1", TokenKind::Gql, TokenState::Invalid);
        assert_eq!(chat_token_kind(&a, &vault), None);
    }
}
