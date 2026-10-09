use cyex_settings::schema::{Account, TokenMeta};
use cyex_twitch::clients;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The two tokens an account can hold (ARCHITECTURE §7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TokenKind {
    /// This app's own client ID: Helix, IRC, EventSub.
    Helix,
    /// A first-party Twitch client ID: GQL-only tools.
    Gql,
}

impl TokenKind {
    pub const ALL: [TokenKind; 2] = [TokenKind::Helix, TokenKind::Gql];

    pub fn key(self) -> &'static str {
        match self {
            TokenKind::Helix => "helix",
            TokenKind::Gql => "gql",
        }
    }

    /// Where a pasted token goes: first-party tokens unlock GQL, anything else is Helix.
    pub fn for_client(client_id: &str) -> Self {
        if clients::is_first_party(client_id) {
            TokenKind::Gql
        } else {
            TokenKind::Helix
        }
    }

    pub fn meta(self, account: &Account) -> Option<&TokenMeta> {
        match self {
            TokenKind::Helix => account.helix.as_ref(),
            TokenKind::Gql => account.gql.as_ref(),
        }
    }

    pub fn meta_mut(self, account: &mut Account) -> &mut Option<TokenMeta> {
        match self {
            TokenKind::Helix => &mut account.helix,
            TokenKind::Gql => &mut account.gql,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_party_tokens_go_to_gql() {
        assert_eq!(TokenKind::for_client(clients::ANDROID_TV), TokenKind::Gql);
        assert_eq!(TokenKind::for_client(clients::WEB), TokenKind::Gql);
        assert_eq!(
            TokenKind::for_client("someappclientid0000000000000"),
            TokenKind::Helix
        );
    }
}
