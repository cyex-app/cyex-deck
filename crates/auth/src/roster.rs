//! Changes to the accounts list (1.3, 1.23). Pure functions; the caller saves the result.

use cyex_settings::schema::{Account, Accounts, TokenMeta, TokenSource};
use cyex_twitch::id::Validation;

use crate::TokenKind;
use crate::inspect::Inspected;
use crate::keeper::expires_at;

/// Adds the account or updates it. The first account becomes the default.
pub fn record_sign_in(
    accounts: &mut Accounts,
    who: &Inspected,
    kind: TokenKind,
    source: TokenSource,
    now: i64,
) {
    let v = &who.validation;
    let meta = meta_from(v, source, now);
    let account = match accounts.get_mut(&v.user_id) {
        Some(account) => account,
        None => {
            accounts.list.push(Account {
                user_id: v.user_id.clone(),
                ..Default::default()
            });
            accounts.list.last_mut().expect("just pushed")
        }
    };
    account.login.clone_from(&v.login);
    account.display_name.clone_from(&who.display_name);
    if !who.avatar.is_empty() {
        account.avatar.clone_from(&who.avatar);
    }
    *kind.meta_mut(account) = Some(meta);
    if accounts.default_account().is_none() {
        accounts.default_id = Some(v.user_id.clone());
    }
}

/// Updates scopes, expiry and login after a check. Returns true if anything changed.
pub fn record_check(
    accounts: &mut Accounts,
    user_id: &str,
    kind: TokenKind,
    v: &Validation,
    now: i64,
) -> bool {
    let Some(account) = accounts.get_mut(user_id) else {
        return false;
    };
    let before = account.clone();
    if !v.login.is_empty() {
        account.login.clone_from(&v.login);
    }
    if let Some(meta) = kind.meta_mut(account) {
        *meta = meta_from(v, meta.source, now);
    }
    *account != before
}

/// Removes one token. An account left with no tokens is removed too.
pub fn remove_token(accounts: &mut Accounts, user_id: &str, kind: TokenKind) {
    if let Some(account) = accounts.get_mut(user_id) {
        *kind.meta_mut(account) = None;
        if account.helix.is_none() && account.gql.is_none() {
            remove_account(accounts, user_id);
        }
    }
}

pub fn remove_account(accounts: &mut Accounts, user_id: &str) {
    accounts.list.retain(|a| a.user_id != user_id);
    if accounts.default_account().is_none() {
        accounts.default_id = accounts.list.first().map(|a| a.user_id.clone());
    }
}

/// Returns false if there's no such account.
pub fn set_default(accounts: &mut Accounts, user_id: &str) -> bool {
    let found = accounts.get(user_id).is_some();
    if found {
        accounts.default_id = Some(user_id.to_owned());
    }
    found
}

fn meta_from(v: &Validation, source: TokenSource, now: i64) -> TokenMeta {
    TokenMeta {
        source,
        scopes: v.scopes.clone().unwrap_or_default(),
        expires_at: expires_at(v.expires_in, now),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;

    fn who(id: &str, login: &str) -> Inspected {
        Inspected {
            validation: Validation {
                client_id: "c".into(),
                login: login.into(),
                user_id: id.into(),
                scopes: Some(vec!["chat:read".into()]),
                expires_in: 14400,
            },
            display_name: login.to_uppercase(),
            avatar: "https://x/a.png".into(),
        }
    }

    #[test]
    fn first_sign_in_becomes_default_and_tokens_merge_by_user_id() {
        let mut a = Accounts::default();
        record_sign_in(
            &mut a,
            &who("1", "one"),
            TokenKind::Helix,
            TokenSource::Device,
            NOW,
        );
        record_sign_in(
            &mut a,
            &who("2", "two"),
            TokenKind::Helix,
            TokenSource::Device,
            NOW,
        );
        record_sign_in(&mut a, &who("1", "one"), TokenKind::Gql, TokenSource::Pasted, NOW);
        assert_eq!(a.list.len(), 2);
        assert_eq!(a.default_id.as_deref(), Some("1"));
        let one = a.get("1").unwrap();
        assert_eq!(one.helix.as_ref().unwrap().expires_at, Some(NOW + 14400));
        assert_eq!(one.gql.as_ref().unwrap().source, TokenSource::Pasted);
        assert_eq!(one.display_name, "ONE");
    }

    #[test]
    fn renames_are_picked_up_on_check() {
        let mut a = Accounts::default();
        record_sign_in(
            &mut a,
            &who("1", "old"),
            TokenKind::Helix,
            TokenSource::Device,
            NOW,
        );
        let renamed = who("1", "new").validation;
        assert!(record_check(&mut a, "1", TokenKind::Helix, &renamed, NOW));
        assert_eq!(a.get("1").unwrap().login, "new");
        assert!(!record_check(&mut a, "1", TokenKind::Helix, &renamed, NOW));
    }

    #[test]
    fn removing_the_last_token_removes_the_account_and_moves_the_default() {
        let mut a = Accounts::default();
        record_sign_in(
            &mut a,
            &who("1", "one"),
            TokenKind::Helix,
            TokenSource::Device,
            NOW,
        );
        record_sign_in(&mut a, &who("2", "two"), TokenKind::Gql, TokenSource::Device, NOW);
        record_sign_in(&mut a, &who("1", "one"), TokenKind::Gql, TokenSource::Device, NOW);
        remove_token(&mut a, "1", TokenKind::Gql);
        assert!(a.get("1").is_some());
        remove_token(&mut a, "1", TokenKind::Helix);
        assert!(a.get("1").is_none());
        assert_eq!(a.default_id.as_deref(), Some("2"));
        assert!(set_default(&mut a, "2"));
        assert!(!set_default(&mut a, "9"));
    }
}
