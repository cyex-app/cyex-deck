//! 1.28 Subscriptions in the app: the account's subs (active and expired), its standing
//! in one channel (sub, resub to share, Prime), sharing a resub in chat, spending Prime,
//! and turning renewal off. Buying, gifting and payment stay on Twitch's own pages (3.8).
//! GQL with the account's token; shapes checked 2026/10/10, mutations never run.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::{Auth, query};
use crate::{Error, Http, Result};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SubChannel")]
pub struct Channel {
    pub id: String,
    pub login: String,
    pub name: String,
    pub avatar: Option<String>,
}

/// A sub the account has now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "MySub")]
pub struct Sub {
    /// Twitch's benefit id, used to stop renewal.
    pub id: String,
    pub channel: Channel,
    /// `1000`, `2000`, `3000`.
    pub tier: String,
    /// RFC 3339; when it runs out if it won't renew.
    pub ends_at: Option<String>,
    /// RFC 3339; the next charge.
    pub renews_at: Option<String>,
    pub prime: bool,
    /// Who gifted it.
    pub gifter: Option<String>,
    /// Renewal was turned off.
    pub canceled: bool,
    /// `WEB`, `IOS`, `ANDROID`; mobile subs are managed in the store app.
    pub platform: String,
    /// Months subscribed in total.
    pub months: Option<u32>,
}

/// A sub that ran out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ExpiredSub")]
pub struct Expired {
    pub channel: Channel,
    pub tier: String,
    pub months: Option<u32>,
    /// RFC 3339.
    pub ended_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "MySubs")]
pub struct Subs {
    pub active: Vec<Sub>,
    pub expired: Vec<Expired>,
    pub has_prime: bool,
}

/// A resub Twitch lets you announce in chat.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ResubShare")]
pub struct Resub {
    /// The token's id; sharing sends this.
    pub id: String,
    pub months: u32,
    /// `None` when Twitch has no streak to show.
    pub streak: Option<u32>,
    pub gift: bool,
}

/// The account in one channel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ChannelSub")]
pub struct ChannelSub {
    pub channel: Channel,
    pub sub: Option<Sub>,
    pub resub: Option<Resub>,
    /// A free Prime sub can go to this channel now.
    pub can_prime: bool,
    /// The channel sells subs at all.
    pub sells: bool,
}

const BENEFIT: &str = "id tier endsAt renewsAt purchasedWithPrime platform isDNRd \
    gift { isGift gifter { login displayName } }";
const CHANNEL: &str = "id login displayName profileImageURL(width: 70)";

pub async fn mine(http: &Http, auth: Auth<'_>) -> Result<Subs> {
    let q = format!(
        "query {{ currentUser {{ id hasPrime \
         subscriptionBenefits(first: 100, criteria: {{filter: ALL}}) {{ edges {{ node {{ {BENEFIT} \
         user {{ {CHANNEL} self {{ subscriptionTenure(tenureMethod: CUMULATIVE) {{ months }} }} }} }} }} }} \
         expiredSubscriptions(first: 100) {{ edges {{ node {{ tenure {{ months end }} \
         channelOwner {{ {CHANNEL} }} product {{ tier }} }} }} }} }} }}"
    );
    let data: Me<MineUser> = query(http, auth, &q, json!({})).await?;
    let me = data.current_user.ok_or_else(|| api("unauthenticated"))?;
    let active = me
        .subscription_benefits
        .map(|c| c.edges)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|e| {
            let b = e.node?;
            let user = *b.user.clone()?;
            let months = user
                .me
                .as_ref()
                .and_then(|s| s.subscription_tenure.as_ref())
                .and_then(|t| t.months);
            Some(b.into_sub(user.into_channel(), months))
        })
        .collect();
    let expired = me
        .expired_subscriptions
        .map(|c| c.edges)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|e| {
            let n = e.node?;
            Some(Expired {
                channel: n.channel_owner?.into_channel(),
                tier: n.product.and_then(|p| p.tier).unwrap_or_default(),
                months: n.tenure.as_ref().and_then(|t| t.months),
                ended_at: n.tenure.and_then(|t| t.end),
            })
        })
        .collect();
    Ok(Subs {
        active,
        expired,
        has_prime: me.has_prime,
    })
}

/// The account's sub, shareable resub and Prime option in one channel. `None` if the
/// channel is gone.
pub async fn channel(http: &Http, auth: Auth<'_>, login: &str) -> Result<Option<ChannelSub>> {
    let q = format!(
        "query($login: String!) {{ currentUser {{ id hasPrime }} user(login: $login) {{ {CHANNEL} \
         subscriptionProducts {{ id }} self {{ subscriptionBenefit {{ {BENEFIT} }} \
         subscriptionTenure(tenureMethod: CUMULATIVE) {{ months }} canPrimeSubscribe \
         resubNotification {{ id cumulativeTenureMonths streakTenureMonths isGiftSubscription }} }} }} }}"
    );
    let data: ChannelData = query(http, auth, &q, json!({ "login": login })).await?;
    let has_prime = data.current_user.ok_or_else(|| api("unauthenticated"))?.has_prime;
    let Some(user) = data.user else { return Ok(None) };
    let sells = user.subscription_products.as_ref().is_some_and(|p| !p.is_empty());
    let me = user.me.clone().unwrap_or_default();
    let channel = user.into_channel();
    let months = me.subscription_tenure.as_ref().and_then(|t| t.months);
    Ok(Some(ChannelSub {
        sub: me
            .subscription_benefit
            .map(|b| b.into_sub(channel.clone(), months)),
        resub: me.resub_notification.map(|r| Resub {
            id: r.id,
            months: r.cumulative_tenure_months.unwrap_or(0),
            streak: r.streak_tenure_months.filter(|s| *s > 0),
            gift: r.is_gift_subscription,
        }),
        can_prime: has_prime && me.can_prime_subscribe,
        sells,
        channel,
    }))
}

/// Announces the resub in the channel's chat, with an optional message.
pub async fn share_resub(
    http: &Http,
    auth: Auth<'_>,
    channel_login: &str,
    token_id: &str,
    message: &str,
    streak: bool,
) -> Result<()> {
    let q = "mutation($input: UseChatNotificationTokenInput!) { useChatNotificationToken(input: $input) { isSuccess } }";
    let vars = json!({ "input": {
        "channelLogin": channel_login, "tokenID": token_id, "message": message, "includeStreak": streak,
    } });
    let data: Value = query(http, auth, q, vars).await?;
    if data["useChatNotificationToken"]["isSuccess"].as_bool() == Some(true) {
        Ok(())
    } else {
        Err(api("NOT_SHARED"))
    }
}

/// Spends the free Prime sub on a channel.
pub async fn prime(http: &Http, auth: Auth<'_>, user_id: &str, channel_id: &str) -> Result<()> {
    let q = "mutation($b: String!, $u: ID!) { spendSubscriptionCredit(input: {broadcasterID: $b, userID: $u}) { \
        userID error { code } } }";
    let data: Value = query(http, auth, q, json!({ "b": channel_id, "u": user_id })).await?;
    match data["spendSubscriptionCredit"]["error"]["code"].as_str() {
        Some(code) => Err(api(code)),
        None => Ok(()),
    }
}

/// Turns renewal off; the sub runs until its end date.
pub async fn stop_renewal(http: &Http, auth: Auth<'_>, user_id: &str, sub_id: &str) -> Result<()> {
    let q = "mutation($u: ID!, $s: ID!) { doNotRenewSubscription(input: {userID: $u, subscriptionID: $s, \
        reasonCode: OTHER}) { subscriptionID errorCode } }";
    let data: Value = query(http, auth, q, json!({ "u": user_id, "s": sub_id })).await?;
    match data["doNotRenewSubscription"]["errorCode"].as_str() {
        Some(code) if !code.is_empty() => Err(api(code)),
        _ => Ok(()),
    }
}

/// Twitch's pages for what the API doesn't do (3.8).
pub mod pages {
    pub const WALLET: &str = "https://www.twitch.tv/wallet?tab=payment-methods";
    pub const SUBSCRIPTIONS: &str = "https://www.twitch.tv/subscriptions?tab=paid";
    /// Buy Bits (V107-B); the web client opens this same address.
    pub const BITS: &str = "https://www.twitch.tv/bits";

    /// Subscribe, resub or change tier.
    pub fn subscribe(login: &str) -> String {
        format!("https://www.twitch.tv/subs/{login}")
    }

    pub fn gift(login: &str) -> String {
        format!("https://www.twitch.tv/subs/{login}?gift=true")
    }
}

fn api(code: &str) -> Error {
    Error::Api {
        status: 200,
        message: code.to_owned(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Me<T> {
    current_user: Option<T>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MineUser {
    #[serde(default)]
    has_prime: bool,
    subscription_benefits: Option<Edges<Benefit>>,
    expired_subscriptions: Option<Edges<RawExpired>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChannelData {
    current_user: Option<Prime>,
    user: Option<RawChannel>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Prime {
    #[serde(default)]
    has_prime: bool,
}

#[derive(Deserialize)]
struct Edges<T> {
    #[serde(default = "Vec::new")]
    edges: Vec<Edge<T>>,
}

#[derive(Deserialize)]
struct Edge<T> {
    node: Option<T>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChannel {
    id: String,
    login: String,
    #[serde(default)]
    display_name: String,
    #[serde(rename = "profileImageURL")]
    profile_image_url: Option<String>,
    subscription_products: Option<Vec<Option<Value>>>,
    #[serde(rename = "self")]
    me: Option<ChannelSelf>,
}

impl RawChannel {
    fn into_channel(self) -> Channel {
        Channel {
            id: self.id,
            login: self.login,
            name: self.display_name,
            avatar: self.profile_image_url,
        }
    }
}

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChannelSelf {
    subscription_benefit: Option<Benefit>,
    subscription_tenure: Option<Tenure>,
    #[serde(default)]
    can_prime_subscribe: bool,
    resub_notification: Option<RawResub>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Tenure {
    months: Option<u32>,
    end: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawResub {
    id: String,
    cumulative_tenure_months: Option<u32>,
    streak_tenure_months: Option<u32>,
    #[serde(default)]
    is_gift_subscription: bool,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Benefit {
    id: String,
    #[serde(default)]
    tier: String,
    ends_at: Option<String>,
    renews_at: Option<String>,
    #[serde(default)]
    purchased_with_prime: bool,
    platform: Option<String>,
    #[serde(rename = "isDNRd", default)]
    is_dnrd: bool,
    gift: Option<Gift>,
    user: Option<Box<RawChannel>>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Gift {
    #[serde(default)]
    is_gift: bool,
    gifter: Option<Gifter>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Gifter {
    login: Option<String>,
    display_name: Option<String>,
}

impl Benefit {
    fn into_sub(self, channel: Channel, months: Option<u32>) -> Sub {
        let gifter = self.gift.filter(|g| g.is_gift).map(|g| {
            g.gifter
                .and_then(|u| u.display_name.or(u.login))
                .unwrap_or_else(|| "Anonymous".into())
        });
        Sub {
            id: self.id,
            channel,
            tier: self.tier,
            ends_at: self.ends_at,
            renews_at: self.renews_at,
            prime: self.purchased_with_prime,
            gifter,
            canceled: self.is_dnrd,
            platform: self.platform.unwrap_or_default(),
            months,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawExpired {
    tenure: Option<Tenure>,
    channel_owner: Option<RawChannel>,
    product: Option<Product>,
}

#[derive(Deserialize)]
struct Product {
    tier: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mine_parses() {
        let d: Me<MineUser> = serde_json::from_str(
            r#"{"currentUser":{"id":"1","hasPrime":true,"subscriptionBenefits":{"edges":[{"node":{
                "id":"b1","tier":"1000","endsAt":null,"renewsAt":"2026-11-01T00:00:00Z","purchasedWithPrime":false,
                "platform":"WEB","isDNRd":false,"gift":{"isGift":true,"gifter":{"login":"g","displayName":"G"}},
                "user":{"id":"9","login":"xqc","displayName":"xQc","profileImageURL":"p","self":{"subscriptionTenure":{"months":14}}}}}]},
              "expiredSubscriptions":{"edges":[{"node":{"tenure":{"months":3,"end":"2025-01-01T00:00:00Z"},
                "channelOwner":{"id":"8","login":"a","displayName":"A","profileImageURL":null},"product":{"tier":"2000"}}}]}}}"#,
        )
        .unwrap();
        let me = d.current_user.unwrap();
        let b = me
            .subscription_benefits
            .unwrap()
            .edges
            .into_iter()
            .next()
            .unwrap()
            .node
            .unwrap();
        let user = *b.user.clone().unwrap();
        let sub = b.into_sub(user.into_channel(), Some(14));
        assert_eq!(
            (sub.gifter.as_deref(), sub.months, sub.channel.login.as_str()),
            (Some("G"), Some(14), "xqc")
        );
        assert_eq!(me.expired_subscriptions.unwrap().edges.len(), 1);
    }

    #[test]
    fn channel_parses() {
        let d: ChannelData = serde_json::from_str(
            r#"{"currentUser":{"id":"1","hasPrime":true},"user":{"id":"9","login":"a","displayName":"A",
              "profileImageURL":null,"subscriptionProducts":[{"id":"p"}],"self":{"subscriptionBenefit":null,
              "subscriptionTenure":{"months":5},"canPrimeSubscribe":true,
              "resubNotification":{"id":"r1","cumulativeTenureMonths":5,"streakTenureMonths":0,"isGiftSubscription":false}}}}"#,
        )
        .unwrap();
        let me = d.user.unwrap().me.unwrap();
        assert!(me.can_prime_subscribe);
        assert_eq!(me.resub_notification.unwrap().cumulative_tenure_months, Some(5));
    }
}
