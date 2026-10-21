//! V107-R41 One channel's dashboard over GQL: where calls go (Twitch, or a local mock in
//! tests and mocks builds), as whom, and the one rule for what a mutation's answer means.
//!
//! The rule, for every dashboard write:
//! - `errors[]` from GQL: `Err` (Twitch didn't run it, or failed).
//! - a payload `error { code }` (or an `error` enum string): `Err(Refused(code))`; an
//!   `error` without a code is `UNKNOWN`, as Twitch's dashboard reads it.
//! - a `null` or missing payload: `Err(Refused(NOT_DONE))`, unless the op is in
//!   [`NULL_IS_DONE`].
//!
//! Per op, what Twitch's own dashboard does with a `null` payload (its public bundles in
//! `target/perf-v107e/twjs`, 2026/10/21; docs/research/twitch-api.md):
//!
//! | op | dashboard on `null` | here |
//! |---|---|---|
//! | createCommunityPointsCustomReward, updateCommunityPointsCustomReward, deleteCommunityPointsCustomReward | throws UNKNOWN | NOT_DONE |
//! | updateCommunityPointsCustomRewardRedemptionStatusesByRedemptions | never reads it (optimistic, refetches) | done |
//! | deleteClips | reads `.clips` | NOT_DONE |
//! | updateVideo | checks the scope it got back | NOT_DONE |
//! | updateVideosViewability | checks only `.error` | done (the caller re-reads the scopes) |
//! | deleteVideos | reads `.videos` | NOT_DONE |
//! | updateChatSettings | `!error && chatSettings` | NOT_DONE |
//! | setUnbanRequestsSettings | `settings` or fails | NOT_DONE |
//! | updateAutoModLevels, add/deleteChannel{Blocked,Permitted}Term… | no code found (shapes from validation probes) | NOT_DONE |
//! | unmodUser, removeEditor, unassignChannelRole | Roles page: FATAL | NOT_DONE |
//! | 1.35 ops: setFreeformTags, createVideoBookmark, create/cancel/goRaid, polls, predictions, Stream Together | not checked again | done, as before (1.35) |

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::gql::{Auth, run_at, run_partial_at};
use crate::{Error, Http, Result};

/// Said for a payload that came back empty: no answer is no yes.
pub const NOT_DONE: &str = "NOT_DONE";

/// Mutations whose `null` payload means done (see the table above).
pub(crate) const NULL_IS_DONE: &[&str] = &[
    "updateCommunityPointsCustomRewardRedemptionStatusesByRedemptions",
    "updateVideosViewability",
    // 1.35, behaviour kept.
    "setFreeformTags",
    "createVideoBookmark",
    "createRaid",
    "cancelRaid",
    "goRaid",
    "createPoll",
    "createPredictionEvent",
    "lockPredictionEvent",
    "resolvePredictionEvent",
    "cancelPredictionEvent",
    "updateGuestStarJoinRequestStatus",
    "setGuestStarUserSettings",
    "startGuestStarSession",
    "endGuestStarSession",
    "inviteUserToGuestStarSession",
    "removeGuestStarInvite",
    "assignUserToGuestStarSlot",
    "removeUserFromGuestStarSlot",
];

/// A channel's dashboard: `id` is the channel (your own: the dashboard only runs there),
/// `base` a local mock (`{base}/gql`) or `None` for Twitch.
#[derive(Clone, Copy)]
pub struct Channel<'a> {
    pub http: &'a Http,
    pub auth: Auth<'a>,
    pub base: Option<&'a str>,
    pub id: &'a str,
}

impl Channel<'_> {
    /// A read: its `data` as `T`.
    pub(crate) async fn read<T: DeserializeOwned>(&self, text: &str, vars: Value) -> Result<T> {
        let data = run_at(self.http, self.auth, self.base, text, vars).await?;
        serde_json::from_value(data).map_err(Error::Decode)
    }

    /// A read whose fields are all optional: data wins when `keep` says it's worth it.
    pub(crate) async fn read_partial(
        &self,
        text: &str,
        vars: Value,
        keep: impl FnOnce(&Value) -> bool,
    ) -> Result<Value> {
        run_partial_at(self.http, self.auth, self.base, text, vars, keep).await
    }

    /// A mutation `field(input: $input)`: its payload, or why not (the rule above).
    pub(crate) async fn write(&self, text: &str, field: &str, input: Value) -> Result<Value> {
        write(self.http, self.auth, self.base, text, field, input).await
    }
}

/// [`Channel::write`] for the 1.35 modules that take `(http, auth)`.
pub(crate) async fn write(
    http: &Http,
    auth: Auth<'_>,
    base: Option<&str>,
    text: &str,
    field: &str,
    input: Value,
) -> Result<Value> {
    let data = run_at(http, auth, base, text, json!({ "input": input })).await?;
    payload(data, field)
}

/// The payload under `field`, by the one rule.
pub(crate) fn payload(mut data: Value, field: &str) -> Result<Value> {
    let p = data.get_mut(field).map(Value::take).unwrap_or(Value::Null);
    if p.is_null() {
        return if NULL_IS_DONE.contains(&field) {
            Ok(Value::Null)
        } else {
            Err(Error::Refused(NOT_DONE.into()))
        };
    }
    match p.get("error") {
        None | Some(Value::Null) => Ok(p),
        Some(Value::String(code)) => Err(Error::Refused(code.clone())),
        Some(e) => {
            let code = e
                .get("code")
                .and_then(Value::as_str)
                .filter(|c| !c.is_empty())
                .unwrap_or("UNKNOWN");
            Err(Error::Refused(code.to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refused(r: Result<Value>) -> String {
        match r {
            Err(Error::Refused(c)) => c,
            other => panic!("not a refusal: {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn v107_r41_one_refusal_rule() {
        let ok = payload(json!({ "deleteClips": { "clips": [] } }), "deleteClips").unwrap();
        assert_eq!(ok, json!({ "clips": [] }));
        assert!(payload(json!({ "unmodUser": { "error": null } }), "unmodUser").is_ok());
        assert_eq!(
            refused(payload(
                json!({ "unmodUser": { "error": { "code": "FORBIDDEN" } } }),
                "unmodUser"
            )),
            "FORBIDDEN"
        );
        assert_eq!(
            refused(payload(
                json!({ "unmodUser": { "error": { "code": null } } }),
                "unmodUser"
            )),
            "UNKNOWN"
        );
        assert_eq!(
            refused(payload(
                json!({ "addChannelBlockedTerm": { "error": "DUPLICATE" } }),
                "addChannelBlockedTerm"
            )),
            "DUPLICATE"
        );
        assert_eq!(
            refused(payload(json!({ "deleteClips": null }), "deleteClips")),
            NOT_DONE
        );
        assert_eq!(refused(payload(json!({}), "updateChatSettings")), NOT_DONE);
    }

    #[test]
    fn v107_r41_null_means_done_only_where_listed() {
        for op in NULL_IS_DONE {
            assert_eq!(payload(json!({ *op: null }), op).unwrap(), Value::Null, "{op}");
        }
        for op in [
            "createCommunityPointsCustomReward",
            "updateCommunityPointsCustomReward",
            "deleteCommunityPointsCustomReward",
            "deleteClips",
            "updateVideo",
            "deleteVideos",
            "updateChatSettings",
            "setUnbanRequestsSettings",
            "updateAutoModLevels",
            "addChannelBlockedTerm",
            "addChannelPermittedTerm",
            "deleteChannelBlockedTermByID",
            "deleteChannelPermittedTermByID",
            "unmodUser",
            "removeEditor",
            "unassignChannelRole",
        ] {
            assert_eq!(refused(payload(json!({ op: null }), op)), NOT_DONE, "{op}");
        }
    }
}
