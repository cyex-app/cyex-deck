//! V111-47 (sign-in parity) The two sign-ins an account can have, and the one line the UI
//! gets when an action needs the one that is missing. The texts are a contract with the UI
//! (it shows an Add button for them): use them verbatim.
//!
//! - `Needs::Helix`: the "Twitch sign-in" (Helix OAuth).
//! - `Needs::Gql`: "Extra Twitch features" (the first-party token).

use std::fmt;

/// The text for a missing Twitch sign-in.
pub const NEEDS_HELIX: &str = "Needs the Twitch sign-in.";
/// The text for missing Extra Twitch features.
pub const NEEDS_GQL: &str = "Needs Extra Twitch features.";

/// The sign-in an action needs and the account doesn't have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Needs {
    Helix,
    Gql,
}

impl Needs {
    pub const fn text(self) -> &'static str {
        match self {
            Self::Helix => NEEDS_HELIX,
            Self::Gql => NEEDS_GQL,
        }
    }
}

impl fmt::Display for Needs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text())
    }
}

impl From<Needs> for String {
    fn from(n: Needs) -> Self {
        n.text().to_owned()
    }
}

/// Which way an action goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Helix,
    Gql,
    Needs(Needs),
}

/// V111-47 Own-channel chat commands that have a GQL twin (modes, raid, ads, markers):
/// Helix first while it works; GQL when there's no Helix sign-in or Helix refused the scope,
/// but only on the account's own channel (GQL's behaviour for a mod elsewhere is unverified).
/// Else the account needs the Twitch sign-in.
pub fn route(own_channel: bool, has_helix: bool, has_gql: bool, helix_refused: bool) -> Route {
    match (has_helix && !helix_refused, own_channel && has_gql) {
        (true, _) => Route::Helix,
        (false, true) => Route::Gql,
        // Helix refused and there's no way around it: its own error stands.
        (false, false) if has_helix => Route::Helix,
        (false, false) => Route::Needs(Needs::Helix),
    }
}

/// Whether a Helix status means "this sign-in can't do it" (a token without the scope).
pub fn helix_refused(status: u16) -> bool {
    matches!(status, 401 | 403)
}

/// V111-47 Whether an action with a GQL twin goes over GQL after Helix: `helix` is `None` without
/// a Helix sign-in, else the status of its failure (`None` inside for no status, e.g. no
/// network). Only a missing sign-in or a refused token moves on; any other Helix error stands.
pub fn use_gql(helix: Option<Option<u16>>) -> bool {
    helix.is_none_or(|status| status.is_some_and(helix_refused))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v111_47_gql_follows_only_a_missing_or_refused_helix() {
        assert!(use_gql(None));
        assert!(use_gql(Some(Some(401))) && use_gql(Some(Some(403))));
        assert!(!use_gql(Some(Some(429))) && !use_gql(Some(Some(500))) && !use_gql(Some(None)));
    }

    #[test]
    fn v111_47_the_two_texts_are_the_contract() {
        assert_eq!(Needs::Helix.to_string(), "Needs the Twitch sign-in.");
        assert_eq!(String::from(Needs::Gql), "Needs Extra Twitch features.");
        assert!(!NEEDS_HELIX.contains("Helix") && !NEEDS_GQL.contains("GQL"));
    }

    #[test]
    fn v111_47_helix_goes_first_then_gql_on_the_own_channel() {
        assert_eq!(route(true, true, true, false), Route::Helix);
        assert_eq!(route(false, true, false, false), Route::Helix);
        // No Twitch sign-in: GQL runs it on your own channel.
        assert_eq!(route(true, false, true, false), Route::Gql);
        // Helix refused the scope: same.
        assert_eq!(route(true, true, true, true), Route::Gql);
    }

    #[test]
    fn v111_47_elsewhere_or_without_either_it_says_which_sign_in() {
        assert_eq!(route(false, false, true, false), Route::Needs(Needs::Helix));
        assert_eq!(route(true, false, false, false), Route::Needs(Needs::Helix));
        // Refused with no GQL to fall back on: Helix's own error stands.
        assert_eq!(route(true, true, false, true), Route::Helix);
        assert_eq!(route(false, true, true, true), Route::Helix);
    }

    #[test]
    fn v111_47_only_auth_statuses_count_as_refused() {
        assert!(helix_refused(401) && helix_refused(403));
        assert!(!helix_refused(400) && !helix_refused(429) && !helix_refused(0));
    }
}
