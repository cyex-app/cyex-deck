use cyex_settings::schema::{Account, TokenMeta};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The two tokens an account can hold (ARCHITECTURE §7). One account holds both when
/// both sign-ins are for the same user (V102-19).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TokenKind {
    /// This app's own client ID: Helix, IRC, EventSub. "Sign in with Twitch".
    Helix,
    /// The GQL client ID (`clients::GQL`): GQL-only tools. A code or a pasted token.
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
