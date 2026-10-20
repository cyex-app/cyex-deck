//! V107-R18: "Twitch as account X". One place builds the GQL and Helix auth
//! bundles from an account's token and the client it was issued to.

use cyex_twitch::{gql, helix};

/// GQL auth for a token and the client it was issued to.
pub fn gql_auth<'a>(token: &'a str, client_id: &'a str) -> gql::Auth<'a> {
    gql::Auth {
        client_id,
        token: Some(token),
    }
}

/// GQL auth for an optional `(token, client_id)` pair; signed out reads public data.
pub fn gql_auth_or_public(pair: Option<&(String, String)>) -> gql::Auth<'_> {
    pair.map_or(gql::Auth::PUBLIC, |(t, c)| gql_auth(t, c))
}

/// Helix auth for a token and the client it was issued to.
pub fn helix_auth<'a>(token: &'a str, client_id: &'a str) -> helix::Auth<'a> {
    helix::Auth { client_id, token }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gql_carries_token_and_client() {
        let a = gql_auth("tok", "cid");
        assert_eq!((a.client_id, a.token), ("cid", Some("tok")));
    }

    #[test]
    fn helix_carries_token_and_client() {
        let a = helix_auth("tok", "cid");
        assert_eq!((a.client_id, a.token), ("cid", "tok"));
    }

    #[test]
    fn signed_out_is_public() {
        let a = gql_auth_or_public(None);
        assert!(a.token.is_none());
        assert_eq!(a.client_id, gql::Auth::PUBLIC.client_id);
        let pair = ("t".to_owned(), "c".to_owned());
        let a = gql_auth_or_public(Some(&pair));
        assert_eq!((a.client_id, a.token), ("c", Some("t")));
    }
}
